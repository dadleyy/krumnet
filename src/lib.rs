extern crate async_std;
extern crate chrono;
extern crate chrono_tz;
extern crate http;
extern crate isahc;
extern crate jsonwebtoken as jwt;
extern crate kramer;
extern crate r2d2;
extern crate r2d2_postgres;
extern crate serde;
extern crate serde_json;
extern crate url;
extern crate uuid;

pub mod configuration;
pub mod constants;

use async_std::io::BufReader;
use async_std::net::{TcpListener, TcpStream};
use async_std::prelude::*;
use async_std::task;
use chrono::prelude::*;
use configuration::Configuration;
use http::header::{self, HeaderMap, HeaderName, HeaderValue};
use http::status::StatusCode;
use http::{Method, Request, Response, Uri};
use jwt::{decode, encode, Header, Validation};
use r2d2_postgres::{PostgresConnectionManager, TlsMode};
use serde::{Deserialize, Serialize};
use std::io::{Error, ErrorKind};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use url::{form_urlencoded, Url};

const MAKE_USER: &'static str = r#"
with new_user as (
    insert into users (default_email, name) values ($1, $2) returning id 
) insert into google_accounts (email, name, google_id, user_id) select $3, $4, $5, new_user.id from new_user;
"#;

#[derive(Debug, Serialize, Deserialize)]
struct SessionClaims {
  id: String,
  exp: u64,
}

fn normalize_error<E>(e: E) -> Error
where
  E: std::error::Error,
{
  Error::new(ErrorKind::Other, format!("{}", e))
}

fn parse_header_name(raw_value: &str) -> Result<HeaderName, Error> {
  HeaderName::from_bytes(raw_value.as_bytes()).map_err(normalize_error)
}

fn parse_header_value(raw_value: &str) -> Result<HeaderValue, Error> {
  HeaderValue::from_bytes(raw_value.as_bytes()).map_err(normalize_error)
}

fn parse_header_line(line: String) -> Result<(HeaderName, HeaderValue), Error> {
  let mut bytes = line.split(":");
  match (bytes.next(), bytes.next()) {
    (Some(left), Some(right)) => Ok((parse_header_name(left)?, parse_header_value(right)?)),
    _ => Err(Error::from(ErrorKind::InvalidData)),
  }
}

fn parse_method(raw_value: &str) -> Result<Method, Error> {
  Method::from_bytes(raw_value.as_bytes()).map_err(normalize_error)
}

fn parse_request_path(raw_value: &str) -> Result<Uri, Error> {
  http::Uri::builder()
    .path_and_query(raw_value)
    .build()
    .map_err(normalize_error)
}

fn parse_request_line(line: String) -> Result<(Method, Uri), Error> {
  let mut bytes = line.split_whitespace();
  match (bytes.next(), bytes.next()) {
    (Some(left), Some(right)) => Ok((parse_method(left)?, parse_request_path(right)?)),
    _ => Err(Error::from(ErrorKind::InvalidData)),
  }
}

#[derive(Debug)]
struct RequestHead {
  session_claims: Option<SessionClaims>,
  headers: HeaderMap,
  method: Method,
  uri: Uri,
}

async fn read_head<T>(reader: T, config: &Configuration) -> Result<RequestHead, Error>
where
  T: async_std::io::Read + std::marker::Unpin,
{
  let mut reader = BufReader::new(reader).lines().take(10);
  let mut map = HeaderMap::new();

  let request_line = reader
    .next()
    .await
    .ok_or(Error::from(ErrorKind::InvalidData))??;

  loop {
    match reader.next().await {
      Some(Ok(line)) if line.is_empty() => break,
      Some(Ok(line)) => match parse_header_line(line) {
        Ok((name, value)) => {
          map.insert(name, value);
        }
        _ => {
          return Err(Error::from(ErrorKind::InvalidData));
        }
      },
      None => break,
      Some(Err(e)) => {
        println!("[error] unable to parse");
        return Err(e);
      }
    }
  }

  let mut session_claims: Option<SessionClaims> = None;

  if let Some(value) = map.get(http::header::AUTHORIZATION) {
    let mut normalized = value
      .to_str()
      .map_err(normalize_error)?
      .trim_start()
      .split_whitespace();

    match (normalized.next(), normalized.next()) {
      (Some("Bearer"), Some(token)) => {
        let token_data = decode::<SessionClaims>(
          &token,
          config.session_secret.as_ref(),
          &Validation {
            leeway: 1000,
            ..Validation::default()
          },
        )
        .map_err(normalize_error)?;
        println!(
          "[debug] handling authorized request: {:?}",
          token_data.claims.id
        );

        session_claims = Some(token_data.claims);
      }
      _ => {
        println!("[warning] invalid authorization header value '{:?}'", value);
      }
    }
  }

  let (method, uri) = parse_request_line(request_line)?;
  Ok(RequestHead {
    uri,
    method,
    session_claims,
    headers: map,
  })
}

async fn write_response<T, U>(mut writer: T, response: Response<U>) -> Result<(), Error>
where
  T: async_std::io::Write + std::marker::Unpin,
  U: serde::Serialize,
{
  let (bits, body) = response.into_parts();
  let bytes = format!(
    "{:?} {} {}\r\n",
    http::Version::HTTP_11,
    bits.status.as_str(),
    bits.status.canonical_reason().unwrap_or_default(),
  );

  writer
    .write(bytes.as_bytes())
    .await
    .map_err(normalize_error)?;

  let ch = format!("{}: close\r\n", header::CONNECTION);
  writer.write(ch.as_bytes()).await.map_err(normalize_error)?;

  let ct = format!("{}: {}\r\n", http::header::CONTENT_TYPE, "application/json");
  writer.write(ct.as_bytes()).await.map_err(normalize_error)?;

  let dh = format!(
    "{}: {}\r\n",
    http::header::DATE,
    Utc::now()
      .with_timezone(&chrono_tz::GMT)
      .format("%a, %e %b %Y %H:%M:%S GMT")
      .to_string()
  );

  writer.write(dh.as_bytes()).await.map_err(normalize_error)?;

  let mut data = String::new();

  if let Ok(serialized) = serde_json::to_string(&body) {
    data = serialized;
  }

  if bits.headers.get(header::CONTENT_LENGTH).is_none() {
    let h = format!("{}: {}\r\n", header::CONTENT_LENGTH, data.len());
    writer.write(h.as_bytes()).await.map_err(normalize_error)?;
  }

  let head = bits
    .headers
    .iter()
    .map(|(key, value)| value.to_str().map(|v| format!("{}: {}", key, v)))
    .flatten()
    .collect::<Vec<String>>()
    .join("\r\n");

  let out = format!("{}\r\n\r\n", head);

  writer
    .write(out.as_bytes())
    .await
    .map_err(normalize_error)?;

  writer
    .write(data.as_bytes())
    .await
    .map_err(normalize_error)?;

  writer.flush().await
}

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

fn make_client() -> Result<isahc::HttpClient, Error> {
  isahc::HttpClient::new().map_err(normalize_error)
}

fn make_user(
  user_info: UserInfoPayload,
  record_store: r2d2::Pool<r2d2_postgres::PostgresConnectionManager>,
) -> Result<UserInfoPayload, Error> {
  record_store
    .get()
    .map_err(normalize_error)?
    .execute(
      MAKE_USER,
      &[
        &user_info.email,
        &user_info.name,
        &user_info.email,
        &user_info.name,
        &user_info.sub,
      ],
    )
    .map_err(normalize_error)?;
  Ok(user_info)
}

async fn fetch_info(authorization: TokenExchangePayload) -> Result<UserInfoPayload, Error> {
  let client = make_client()?;
  let mut request = Request::builder();
  let bearer = format!("Bearer {}", authorization.access_token);
  request
    .method(Method::GET)
    .uri(constants::google_info_url())
    .header("Authorization", bearer.as_str());

  match client.send(request.body(()).map_err(normalize_error)?) {
    Ok(mut response) if response.status() == 200 => {
      serde_json::from_reader(response.body_mut()).map_err(normalize_error)
    }
    Ok(response) => Err(Error::new(
      ErrorKind::Other,
      format!("bad response satus from google sso: {}", response.status()),
    )),
    Err(e) => Err(Error::new(ErrorKind::Other, format!("{}", e))),
  }
}

async fn exchange_code(code: &str, config: &Configuration) -> Result<TokenExchangePayload, Error> {
  let client = make_client()?;

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

async fn write_error<T, E>(writer: T, e: E) -> Result<(), Error>
where
  T: async_std::io::Write + std::marker::Unpin,
  E: std::error::Error,
{
  println!("[warning] 400 - {:?}", e);
  let mut out = Response::builder();
  out.status(StatusCode::BAD_REQUEST);

  match out.body(()) {
    Ok(response) => write_response(writer, response).await,
    Err(e) => {
      println!("[warning] problem building response {:?}", e);
      return Err(Error::from(ErrorKind::NotFound));
    }
  }
}

fn preflight() -> Result<Response<()>, Error> {
  let mut out = Response::builder();
  out.status(StatusCode::OK);
  out.body(()).map_err(normalize_error)
}

fn not_found() -> Result<Response<()>, Error> {
  let mut out = Response::builder();
  out.status(StatusCode::NOT_FOUND);
  out.body(()).map_err(normalize_error)
}

fn redirect(location: &str) -> Result<Response<()>, Error> {
  let mut out = Response::builder();
  out
    .status(StatusCode::FOUND)
    .header(http::header::LOCATION, location)
    .body(())
    .map_err(normalize_error)
}

fn with_cors<T>(config: &Configuration, mut response: Response<T>) -> Result<Response<T>, Error> {
  let headers = response.headers_mut();

  headers.insert(
    header::ACCESS_CONTROL_ALLOW_ORIGIN,
    HeaderValue::from_str(&config.krumi.cors_origin).map_err(normalize_error)?,
  );

  headers.insert(
    http::header::ACCESS_CONTROL_MAX_AGE,
    HeaderValue::from_str("3600").map_err(normalize_error)?,
  );

  headers.insert(
    header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
    HeaderValue::from_str("true").map_err(normalize_error)?,
  );

  headers.insert(
    header::ACCESS_CONTROL_ALLOW_HEADERS,
    HeaderValue::from_str(
      "access-control-allow-credentials, access-control-allow-origin, authorization",
    )
    .map_err(normalize_error)?,
  );

  headers.insert(
    header::ACCESS_CONTROL_ALLOW_METHODS,
    HeaderValue::from_str("GET, HEAD, POST, PUT, DELETE, TRACE, OPTIONS, PATCH")
      .map_err(normalize_error)?,
  );

  Ok(response)
}

async fn authenticate(
  uri: Uri,
  config: &Configuration,
  record_store: r2d2::Pool<r2d2_postgres::PostgresConnectionManager>,
) -> Result<Response<()>, Error> {
  let code = match form_urlencoded::parse(uri.query().unwrap_or_default().as_bytes())
    .find(|(key, _)| key == "code")
  {
    Some((_, code)) => code,
    None => return Err(Error::new(ErrorKind::Other, "no code")),
  };

  let authorization = match exchange_code(&code, config).await {
    Ok(token) => token,
    Err(e) => {
      println!("[warning] unable to exchange code: {:?}", e);
      return Err(Error::new(ErrorKind::Other, "invalid code"));
    }
  };

  let user_info = fetch_info(authorization).await.map_err(normalize_error)?;

  let tenant = record_store
    .get()
    .map_err(normalize_error)?
    .query(
      "select u.id from users as u inner join google_accounts as g on g.user_id = u.id where google_id = $1 limit 1",
      &[&user_info.sub],
    )?;

  match tenant.iter().nth(0) {
    Some(u) => {
      if let Some(Ok(v)) = u.get_opt::<usize, String>(0) {
        println!("[debg] found user: {:?}", v);
      }
    }
    None => {
      println!("[debg] unable to find user, creating");
      make_user(user_info.clone(), record_store)?;
    }
  };

  println!(
    "[debug] connecting to redis - {}",
    config.session_store.redis_uri.as_str()
  );
  let con = std::net::TcpStream::connect(config.session_store.redis_uri.as_str())?;
  let id = uuid::Uuid::new_v4();
  let data = serde_json::to_string(&user_info).map_err(normalize_error)?;
  let set = kramer::StringCommand::Set(
    kramer::Arity::One((format!("session:{}", id), data)),
    None,
    kramer::Insertion::Always,
  );
  println!("[debug] sending session set cmd");
  let response = kramer::execute(con, set).map_err(normalize_error)?;
  println!("[debug] command successful, response: {:?}", response);

  let exp = (std::time::SystemTime::now() + std::time::Duration::from_secs(60 * 60 * 24))
    .duration_since(std::time::UNIX_EPOCH)
    .map_err(normalize_error)?
    .as_secs();

  let claims = SessionClaims {
    exp,
    id: format!("{}", id),
  };

  let token =
    encode(&Header::default(), &claims, config.session_secret.as_ref()).map_err(normalize_error)?;

  println!(
    "[debug] session {:?} created, stored user: {:?} ({:?})",
    id, user_info, token
  );

  let mut location = Url::parse(config.krumi.auth_uri.as_str()).map_err(normalize_error)?;

  location.query_pairs_mut().clear().append_pair(
    constants::KRUMI_SESSION_ID_KEY,
    format!("{}", token).as_str(),
  );

  redirect(location.as_str())
}

fn identify(
  head: &RequestHead,
  config: &Configuration,
) -> Result<Response<UserInfoPayload>, Error> {
  let session_id = match &head.session_claims {
    Some(claims) => claims.id.as_str(),
    None => return Err(Error::new(ErrorKind::Other, "no session id")),
  };

  println!("[debug] finding session id {}", session_id);

  let con = std::net::TcpStream::connect(config.session_store.redis_uri.as_str())?;
  let finder =
    kramer::StringCommand::Get::<_, bool>(kramer::Arity::One(format!("session:{}", session_id)));
  let response = kramer::execute(con, finder).map_err(normalize_error)?;

  let user_info = match response {
    kramer::Response::Item(kramer::ResponseValue::String(value)) => {
      println!("[debug] pulled session data, {}", value.as_str());
      serde_json::from_str(value.as_str()).map_err(normalize_error)?
    }
    _ => return Err(Error::new(ErrorKind::Other, "cannot connect to redis")),
  };

  println!("[debug] session info lookup complete; found user info");

  Response::builder()
    .status(StatusCode::OK)
    .body(user_info)
    .map_err(normalize_error)
}

fn login(config: &Configuration) -> Result<Response<()>, Error> {
  let mut location = Url::parse(constants::google_auth_url().as_str()).map_err(normalize_error)?;

  println!("[debug] login attempt, building redir");

  location
    .query_pairs_mut()
    .clear()
    .append_pair(
      constants::GOOGLE_AUTH_RESPONSE_TYPE_KEY,
      constants::GOOGLE_AUTH_RESPONSE_TYPE_VALUE,
    )
    .append_pair(
      constants::GOOGLE_AUTH_CLIENT_ID_KEY,
      &config.google.client_id,
    )
    .append_pair(
      constants::GOOGLE_AUTH_REDIRECT_URI_KEY,
      &config.google.redirect_uri,
    )
    .append_pair(
      constants::GOOGLE_AUTH_SCOPE_KEY,
      constants::GOOGLE_AUTH_SCOPE_VALUE,
    );

  redirect(location.as_str())
}

async fn forget(head: &RequestHead, config: &Configuration) -> Result<Response<()>, Error> {
  let token = match form_urlencoded::parse(head.uri.query().unwrap_or_default().as_bytes())
    .find(|(key, _)| key == "session_id")
  {
    Some((_, code)) => code,
    None => return Err(Error::new(ErrorKind::Other, "no session id")),
  };

  let token_data = decode::<SessionClaims>(
    &token,
    config.session_secret.as_ref(),
    &Validation {
      leeway: 1000,
      ..Validation::default()
    },
  )
  .map_err(normalize_error)?;

  let session_id = token_data.claims.id;

  let con = std::net::TcpStream::connect(config.session_store.redis_uri.as_str())?;

  kramer::execute(
    con,
    kramer::Command::Del::<_, bool>(kramer::Arity::One(format!("session:{}", session_id))),
  )
  .map_err(normalize_error)?;

  redirect(&config.krumi.auth_uri)
}

async fn handle<T>(
  stream: TcpStream,
  configuration: T,
  record_store: r2d2::Pool<r2d2_postgres::PostgresConnectionManager>,
) -> Result<(), Error>
where
  T: std::convert::AsRef<Configuration>,
{
  let config = configuration.as_ref();

  let head = match read_head(&stream, config).await {
    Ok(v) => v,
    Err(e) => {
      println!("[warning] unable to parse headers: {:?}", e);
      return Err(e);
    }
  };

  match (head.method.clone(), head.uri.path()) {
    (Method::OPTIONS, _) => {
      println!("[debug] preflight request for {:?}", head.uri);
      match preflight().and_then(|r| with_cors(config, r)) {
        Ok(response) => write_response(&stream, response).await?,
        Err(e) => write_error(&stream, e).await?,
      }
    }
    (Method::GET, "/auth/redirect") => match login(config) {
      Ok(response) => write_response(&stream, response).await?,
      Err(e) => write_error(&stream, e).await?,
    },
    (Method::GET, "/auth/callback") => match authenticate(head.uri, config, record_store).await {
      Ok(response) => write_response(&stream, response).await?,
      Err(e) => write_error(&stream, e).await?,
    },
    (Method::GET, "/auth/forget") => match forget(&head, config).await {
      Ok(response) => write_response(&stream, response).await?,
      Err(e) => write_error(&stream, e).await?,
    },
    (Method::GET, "/auth/identify") => {
      println!("[debug] identify request for {:?}", head.uri);
      match identify(&head, config).and_then(|r| with_cors(config, r)) {
        Ok(response) => write_response(&stream, response).await?,
        Err(e) => write_error(&stream, e).await?,
      }
    }
    _ => {
      println!("[debug] 404 for {:?}", head.uri);
      write_response(&stream, not_found()?).await?;
    }
  }

  drop(stream);
  Ok(())
}

async fn broker_loop(chan: Receiver<String>) {
  println!("[debug] starting broker event loop");

  for msg in chan.iter() {
    println!("[debug] broker has message: {:?}", msg);
  }
}

pub async fn run(configuration: Configuration) -> Result<(), Box<dyn std::error::Error>> {
  let listener = TcpListener::bind(&configuration.addr).await?;
  let mut incoming = listener.incoming();
  let (sender, receiver) = channel::<String>();
  let db = r2d2::Pool::builder()
    .connection_timeout(std::time::Duration::new(1, 0))
    .build(PostgresConnectionManager::new(
      configuration.record_store.postgres_uri.as_str(),
      TlsMode::None,
    )?)?;

  let shared_config = Arc::from(configuration);

  let broker = task::spawn(broker_loop(receiver));

  while let Some(stream) = incoming.next().await {
    match stream {
      Ok(connection) => {
        let local_config = shared_config.clone();
        let local_db = db.clone();
        task::spawn(handle(connection, local_config, local_db));
      }
      Err(e) => {
        println!("[warning] invalid connection: {:?}", e);
        continue;
      }
    }
  }

  drop(sender);
  broker.await;

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use mockito::{mock, Matcher};

  #[test]
  fn test_token_exchange_success() {
    let mocked = mock("POST", Matcher::Any)
      .with_status(200)
      .with_body(r#"{"access_token": "access-token"}"#)
      .create();

    let result = task::block_on(async {
      let config = Configuration::default();
      let code = "";
      exchange_code(code, &config).await
    });

    assert_eq!(
      result.unwrap(),
      TokenExchangePayload {
        access_token: String::from("access-token")
      }
    );
    drop(mocked);
  }

  #[test]
  fn test_token_exchange_fail() {
    let mocked = mock("POST", Matcher::Any).with_status(400).create();

    let result = task::block_on(async {
      let config = Configuration::default();
      let code = "";
      exchange_code(code, &config).await
    });

    assert_eq!(result.is_err(), true);
    drop(mocked);
  }
}
