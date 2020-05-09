use isahc::HttpClient;
use log::info;
use r2d2_postgres::postgres::row::Row;
use serde::{Deserialize, Serialize};
use std::io::{Error, ErrorKind};

use crate::authorization::{cors as cors_headers, Authorization, AuthorizationUrls};
use crate::configuration::GoogleCredentials;
use crate::http::{query as qs, Builder, Method, Request, Response as Res, Uri, Url};
use crate::persistence::{Connection as RecordConnection, RecordStore};
use crate::session::SessionStore;

const USER_FOR_SESSION: &'static str = include_str!("data-store/load-user-for-session.sql");
const FIND_USER: &'static str = include_str!("data-store/find-user-by-google-id.sql");
const CREATE_USER: &'static str = include_str!("data-store/create-user.sql");

#[derive(Debug, PartialEq, Deserialize)]
struct TokenExchangePayload {
  access_token: String,
}

#[derive(Debug, Clone, Deserialize, Default, Serialize)]
struct UserInfoPayload {
  name: String,
  sub: String,
  email: String,
  picture: String,
}

pub async fn destroy(
  auth: &Option<Authorization>,
  uri: &Uri,
  session: &SessionStore,
  urls: &AuthorizationUrls,
) -> Result<Res<()>, Error> {
  let token = auth
    .as_ref()
    .map(|Authorization(_, _, _, token)| token.clone())
    .unwrap_or(
      uri
        .query()
        .and_then(|q| qs::parse(q.as_bytes()).find(|(k, _k)| k == "token"))
        .map(|(_k, v)| v.into_owned())
        .unwrap_or_default(),
    );

  info!("destroying session from token: {}", token);
  session.destroy(&token).await?;

  Ok(Res::redirect(&urls.callback))
}

// Given the token returned from an oauth code exchange, load the user's information from the
// google api.
async fn fetch_info(
  authorization: TokenExchangePayload,
  urls: &AuthorizationUrls,
) -> Result<UserInfoPayload, Error> {
  let client = HttpClient::new().map_err(|e| Error::new(ErrorKind::Other, e))?;
  let bearer = format!("Bearer {}", authorization.access_token);

  let request = Request::builder()
    .method(Method::GET)
    .uri(&urls.identify)
    .header("Authorization", bearer.as_str())
    .body(())
    .map_err(|e| Error::new(ErrorKind::Other, e))?;

  match client.send(request) {
    Ok(mut response) if response.status() == 200 => {
      serde_json::from_reader(response.body_mut()).map_err(|e| Error::new(ErrorKind::Other, e))
    }
    Ok(response) => Err(Error::new(
      ErrorKind::Other,
      format!("bad response satus from google sso: {}", response.status()),
    )),
    Err(e) => Err(Error::new(ErrorKind::Other, format!("{}", e))),
  }
}

// Given an oauth code returned from google, attempt to exchange the code for a real auth token
// that will provide access to user information.
async fn exchange_code(
  code: &str,
  authorization: &AuthorizationUrls,
) -> Result<TokenExchangePayload, Error> {
  let client = HttpClient::new().map_err(|e| Error::new(ErrorKind::Other, e))?;
  let (
    exchange_url,
    GoogleCredentials {
      client_id,
      client_secret,
      redirect_uri,
    },
  ) = &authorization.exchange;

  let encoded = qs::Serializer::new(String::new())
    .append_pair("code", code)
    .append_pair("client_id", client_id.as_str())
    .append_pair("client_secret", client_secret.as_str())
    .append_pair("redirect_uri", redirect_uri.as_str())
    .append_pair("grant_type", "authorization_code")
    .finish();

  match client.post(exchange_url, encoded) {
    Ok(mut response) if response.status().is_success() => {
      let body = response.body_mut();
      let payload = match serde_json::from_reader(body) {
        Ok(p) => p,
        Err(e) => {
          return Err(Error::new(
            ErrorKind::Other,
            format!("unable to parse response body: {:?}", e),
          ));
        }
      };
      Ok(payload)
    }
    Ok(response) => Err(Error::new(
      ErrorKind::Other,
      format!("bad response from google sso: {:?}", response.status()),
    )),
    Err(e) => Err(Error::new(
      ErrorKind::Other,
      format!("unable to send code to google sso: {:?}", e),
    )),
  }
}

// Given user information loaded from the api, attempt to save the information into the persistence
// engine.
fn make_user(details: &UserInfoPayload, conn: &mut RecordConnection) -> Result<String, Error> {
  let UserInfoPayload {
    email,
    name,
    sub,
    picture: _,
  } = details;
  conn
    .execute(CREATE_USER, &[&email, &name, &email, &name, &sub])
    .map_err(|e| Error::new(ErrorKind::Other, e))?;

  let tenant = conn
    .query(FIND_USER, &[&sub])
    .map_err(|e| Error::new(ErrorKind::Other, e))?;

  match tenant.iter().nth(0) {
    Some(row) => match row.try_get::<_, String>(0) {
      Ok(id) => Ok(id),
      _ => Err(Error::new(
        ErrorKind::Other,
        "Found matching row, but unable to parse",
      )),
    },
    _ => Err(Error::new(
      ErrorKind::Other,
      "Unable to find previously inserted user",
    )),
  }
}

// Attempt to find a user based on the google account id returned. If none is found, attempt to
// find by the email address and make sure to backfill the google account. If there is still no
// matching user information, attempt to create a new user and google account.
fn find_or_create_user(profile: &UserInfoPayload, records: &RecordStore) -> Result<String, Error> {
  let mut conn = records.get()?;
  info!("loaded user info: {:?}", profile);

  let tenant = conn
    .query(FIND_USER, &[&profile.sub])
    .map_err(|e| Error::new(ErrorKind::Other, e))?;

  match tenant.iter().nth(0) {
    Some(row) => match row.try_get::<_, String>(0) {
      Ok(id) => {
        info!("found existing user {}", id);
        Ok(id)
      }
      _ => Err(Error::new(
        ErrorKind::Other,
        "Unable to parse a valid id from matching row",
      )),
    },
    None => {
      info!("no matching user, creating");
      make_user(&profile, &mut conn)
    }
  }
}

fn build_krumi_callback(urls: &AuthorizationUrls, token: &String) -> Result<String, Error> {
  let mut parsed_callback =
    Url::parse(&urls.callback).map_err(|e| Error::new(ErrorKind::Other, e))?;

  parsed_callback
    .query_pairs_mut()
    .append_pair("token", token);

  Ok(parsed_callback.into_string())
}

// This is the route handler that is used as the redirect uri of the google client. It is
// responsible for receiving the code from the successful oauth prompt and redirecting the user to
// the krumpled ui.
pub async fn callback(
  uri: Uri,
  session: &SessionStore,
  records: &RecordStore,
  authorization: &AuthorizationUrls,
) -> Result<Res<()>, Error> {
  let query = uri.query().unwrap_or_default().as_bytes();

  let code = match qs::parse(query).find(|(key, _)| key == "code") {
    Some((_, code)) => code,
    None => return Ok(Res::not_found(None)),
  };

  let payload = match exchange_code(&code, authorization).await {
    Ok(payload) => payload,
    Err(e) => {
      info!("[warning] unable ot exchange code: {}", e);
      return Ok(Res::not_found(None));
    }
  };

  let profile = match fetch_info(payload, authorization).await {
    Ok(info) => info,
    Err(e) => {
      info!("[warning] unable to fetch user info: {}", e);
      return Ok(Res::not_found(None));
    }
  };

  let uid = match find_or_create_user(&profile, records) {
    Ok(id) => id,
    Err(e) => {
      info!("[warning] unable to create/find user: {:?}", e);
      return Ok(Res::not_found(None));
    }
  };

  let token = session.create(&uid).await?;
  info!("created session for token '{}'", token);

  build_krumi_callback(authorization, &token).map(|redir| Res::redirect(&redir))
}

#[derive(Debug, Serialize)]
pub struct SessionPayload {
  pub id: String,
  pub email: String,
  pub name: String,
}

pub fn parse_user_session_query(row: Row) -> Option<SessionPayload> {
  row.try_get(0).ok().and_then(|id| {
    row.try_get(1).ok().and_then(|name| {
      row
        .try_get(2)
        .ok()
        .map(|email| SessionPayload { id, email, name })
    })
  })
}

pub async fn identify(
  authorization: &Option<Authorization>,
  records: &RecordStore,
  auth_urls: &AuthorizationUrls,
) -> Result<Res<SessionPayload>, Error> {
  let Authorization(uid, _, _, _) = match authorization {
    Some(auth) => auth,
    None => return Ok(Res::not_found(cors_headers(auth_urls).ok())),
  };

  let mut conn = records.get()?;

  let tenant = conn
    .query(USER_FOR_SESSION, &[&uid])
    .ok()
    .and_then(|mut rows| rows.pop())
    .and_then(parse_user_session_query);

  info!(
    "loading session payload for user '{}' (payload {:?})",
    uid, tenant
  );

  tenant
    .and_then(|found| {
      let mut builder = Builder::new().status(200);
      let cors = cors_headers(&auth_urls).ok()?;

      for header in cors {
        if let (Some(key), value) = header {
          builder = builder.header(key, value);
        }
      }

      builder.body(found).map(|res| Ok(Res::json(res))).ok()
    })
    .unwrap_or(Ok(Res::not_found(None)))
}

#[cfg(test)]
mod test {
  use crate::configuration::test_helpers::load_config;
  use crate::persistence::RecordStore;

  #[test]
  fn existing_user_ok() {
    let config = load_config();
    println!("config: {:?}", config);
    assert!(config.is_ok());
    let unwrapped = config.unwrap();
    let records = RecordStore::open(&unwrapped);
    println!("records: {:?}", records);
    assert!(records.is_ok());
  }
}
