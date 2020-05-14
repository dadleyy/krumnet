use isahc::HttpClient;
use log::{debug, info, warn};
use serde::Deserialize;
use std::io::{Error, ErrorKind, Result};

use crate::configuration::GoogleCredentials;
use crate::constants::{
  google_auth_url, google_info_url, google_token_url, GOOGLE_AUTH_CLIENT_ID_KEY,
  GOOGLE_AUTH_REDIRECT_URI_KEY, GOOGLE_AUTH_RESPONSE_TYPE_KEY, GOOGLE_AUTH_RESPONSE_TYPE_VALUE,
  GOOGLE_AUTH_SCOPE_KEY, GOOGLE_AUTH_SCOPE_VALUE,
};
use crate::http::{header, query as qs, Method, Request, Response, Uri, Url};
use crate::{errors, Context};

const FIND_USER: &'static str = include_str!("data-store/find-user-by-google-id.sql");
const CREATE_USER: &'static str = include_str!("data-store/create-user.sql");

// A TokenExchangePayload represents the response received from google oauth that contains the
// authentication token that will be used in subsequent requests on behalf of this user.
#[derive(Debug, PartialEq, Deserialize)]
struct TokenExchangePayload {
  access_token: String,
}

// The UserInfoPayload represents the data received from the google profile api.
#[derive(Debug, Clone, Deserialize, Default)]
struct UserInfoPayload {
  name: String,
  sub: String,
  email: String,
  picture: String,
}

async fn exchange_code(code: &str, context: &Context) -> Result<TokenExchangePayload> {
  let client = HttpClient::new().map_err(errors::humanize_error)?;
  let GoogleCredentials {
    client_id,
    client_secret,
    redirect_uri,
  } = &context.config().google;

  let encoded = qs::Serializer::new(String::new())
    .append_pair("code", code)
    .append_pair("client_id", client_id.as_str())
    .append_pair("client_secret", client_secret.as_str())
    .append_pair("redirect_uri", redirect_uri.as_str())
    .append_pair("grant_type", "authorization_code")
    .finish();

  match client.post(google_token_url(), encoded) {
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

// Given the token returned from an oauth code exchange, load the user's information from the
// google api.
async fn fetch_info(info: TokenExchangePayload) -> Result<UserInfoPayload> {
  let client = HttpClient::new().map_err(errors::humanize_error)?;
  let bearer = format!("Bearer {}", info.access_token);

  let request = Request::builder()
    .method(Method::GET)
    .uri(google_info_url())
    .header(header::AUTHORIZATION, bearer.as_str())
    .body(())
    .map_err(errors::humanize_error)?;

  match client.send(request) {
    Ok(mut response) if response.status() == 200 => {
      serde_json::from_reader(response.body_mut()).map_err(errors::humanize_error)
    }
    Ok(response) => Err(Error::new(
      ErrorKind::Other,
      format!("bad response satus from google sso: {}", response.status()),
    )),
    Err(e) => Err(Error::new(ErrorKind::Other, format!("{}", e))),
  }
}

// Given user information loaded from the api, attempt to save the information into the persistence
// engine, returning the newly created system id if successful.
fn make_user(details: &UserInfoPayload, context: &Context) -> Result<String> {
  let UserInfoPayload {
    email,
    name,
    sub,
    picture: _,
  } = details;

  context
    .records()
    .execute(CREATE_USER, &[&email, &name, &email, &name, &sub])?;

  let tenant = context.records().query(FIND_USER, &[&sub])?;

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
fn find_or_create_user(profile: &UserInfoPayload, context: &Context) -> Result<String> {
  let tenant = context.records().query(FIND_USER, &[&profile.sub])?;

  info!("loaded user info: {:?}", profile);

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
      make_user(&profile, context)
    }
  }
}

fn build_krumi_callback(context: &Context, token: &String) -> Result<String> {
  let mut parsed_callback =
    Url::parse(&context.config().krumi.auth_uri).map_err(errors::humanize_error)?;

  parsed_callback
    .query_pairs_mut()
    .append_pair("token", token);

  Ok(parsed_callback.into_string())
}

pub async fn callback(context: &Context, uri: &Uri) -> Result<Response> {
  let query = uri.query().unwrap_or_default().as_bytes();

  let code = match qs::parse(query).find(|(key, _)| key == "code") {
    Some((_, code)) => code,
    None => return Ok(Response::not_found()),
  };

  let payload = match exchange_code(&code, context).await {
    Ok(payload) => payload,
    Err(e) => {
      warn!("[warning] unable ot exchange code: {}", e);
      return Ok(Response::not_found());
    }
  };

  let profile = match fetch_info(payload).await {
    Ok(info) => info,
    Err(e) => {
      warn!("[warning] unable to fetch user info: {}", e);
      return Ok(Response::not_found());
    }
  };

  info!("received oauth callback - {:?}", profile);

  let uid = match find_or_create_user(&profile, context) {
    Ok(id) => id,
    Err(e) => {
      info!("[warning] unable to create/find user: {:?}", e);
      return Ok(Response::not_found());
    }
  };

  let token = context.session().create(&uid).await?;
  info!("created session for token '{}'", token);

  build_krumi_callback(context, &token).map(|redir| Response::redirect(&redir))
}

pub fn redirect(context: &Context) -> Result<Response> {
  let configuration = context.config();
  let mut url = google_auth_url()
    .parse::<Url>()
    .map_err(errors::humanize_error)?;

  url
    .query_pairs_mut()
    .clear()
    .append_pair(
      GOOGLE_AUTH_RESPONSE_TYPE_KEY,
      GOOGLE_AUTH_RESPONSE_TYPE_VALUE,
    )
    .append_pair(GOOGLE_AUTH_CLIENT_ID_KEY, &configuration.google.client_id)
    .append_pair(
      GOOGLE_AUTH_REDIRECT_URI_KEY,
      &configuration.google.redirect_uri,
    )
    .append_pair(GOOGLE_AUTH_SCOPE_KEY, GOOGLE_AUTH_SCOPE_VALUE);

  debug!("oauth flow redirect to {:?}", url);

  Ok(Response::redirect(&url))
}
