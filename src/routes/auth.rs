use http::response::{Builder, Response};
use http::status::StatusCode;
use http::Uri;
use http::{Method, Request};
use r2d2::PooledConnection;
use r2d2_postgres::PostgresConnectionManager as Postgres;
use serde::{Deserialize, Serialize};
use std::io::{Error, ErrorKind};
use url::form_urlencoded;

use crate::configuration::Configuration;
use crate::constants;
use crate::persistence::RecordStore;

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

async fn fetch_info(authorization: TokenExchangePayload) -> Result<UserInfoPayload, Error> {
  let client = isahc::HttpClient::new().map_err(|e| Error::new(ErrorKind::Other, e))?;
  let mut request = Request::builder();
  let bearer = format!("Bearer {}", authorization.access_token);
  request
    .method(Method::GET)
    .uri(constants::google_info_url())
    .header("Authorization", bearer.as_str());

  match client.send(
    request
      .body(())
      .map_err(|e| Error::new(ErrorKind::Other, e))?,
  ) {
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

async fn exchange_code(code: &str, config: &Configuration) -> Result<TokenExchangePayload, Error> {
  let client = isahc::HttpClient::new().map_err(|e| Error::new(ErrorKind::Other, e))?;

  let encoded: String = form_urlencoded::Serializer::new(String::new())
    .append_pair("code", code)
    .append_pair("client_id", &config.google.client_id)
    .append_pair("client_secret", &config.google.client_secret)
    .append_pair("redirect_uri", &config.google.redirect_uri)
    .append_pair("grant_type", "authorization_code")
    .finish();

  match client.post(constants::google_token_url(), encoded) {
    Ok(mut response) if response.status() == StatusCode::OK => {
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

fn make_user(details: UserInfoPayload, conn: PooledConnection<Postgres>) -> Result<(), Error> {
  let query = include_str!("data-store/create_user.sql");
  let UserInfoPayload {
    email,
    name,
    sub,
    picture: _,
  } = details;
  conn
    .execute(query, &[&email, &name, &email, &name, &sub])
    .map_err(|e| Error::new(ErrorKind::Other, e))?;
  Ok(())
}

pub async fn callback(
  uri: Uri,
  config: &Configuration,
  records: &RecordStore,
) -> Result<Response<Option<u8>>, Error> {
  let query = uri.query().unwrap_or_default().as_bytes();
  let code = match form_urlencoded::parse(query).find(|(key, _)| key == "code") {
    Some((_, code)) => code,
    None => {
      return Builder::new()
        .status(404)
        .body(None)
        .map_err(|e| Error::new(ErrorKind::Other, e))
    }
  };

  let payload = match exchange_code(&code, config).await {
    Ok(payload) => payload,
    Err(e) => {
      println!("[warning] unable ot exchange code: {}", e);
      return Builder::new()
        .status(404)
        .body(None)
        .map_err(|e| Error::new(ErrorKind::Other, e));
    }
  };

  let profile = match fetch_info(payload).await {
    Ok(info) => info,
    Err(e) => {
      println!("[warning] unable to fetch user info: {}", e);
      return Builder::new()
        .status(404)
        .body(None)
        .map_err(|e| Error::new(ErrorKind::Other, e));
    }
  };

  let conn = records.get()?;
  println!("[debug] loaded user info: {:?}", profile);
  let find_user = include_str!("data-store/find_user.sql");
  let tenant = conn.query(find_user, &[&profile.sub])?;

  match tenant.iter().nth(0) {
    Some(_) => println!("[debug] found matching user"),
    None => {
      println!("[debug] no matching user, creating");
      make_user(profile, conn)?;
    }
  }

  Builder::new()
    .status(200)
    .body(None)
    .map_err(|e| Error::new(ErrorKind::Other, e))
}
