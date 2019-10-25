extern crate async_std;
extern crate chrono;
extern crate chrono_tz;
extern crate http;
extern crate isahc;
extern crate redis;
extern crate serde;
extern crate serde_json;
extern crate url;

pub mod configuration;
pub mod constants;

use async_std::io::BufReader;
use async_std::net::{TcpListener, TcpStream};
use async_std::prelude::*;
use async_std::task;
use chrono::prelude::*;
use configuration::Configuration;
use constants::GOOGLE_AUTH_URL;
use http::header::{HeaderMap, HeaderName, HeaderValue};
use http::status::StatusCode;
use http::{Method, Request, Response, Uri};
use serde::Deserialize;
use std::io::{Error, ErrorKind};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use url::{form_urlencoded, Url};

fn parse_header_name(raw_value: &str) -> Result<HeaderName, Error> {
  HeaderName::from_bytes(raw_value.as_bytes()).map_err(|_e| Error::from(ErrorKind::InvalidData))
}

fn parse_header_value(raw_value: &str) -> Result<HeaderValue, Error> {
  HeaderValue::from_bytes(raw_value.as_bytes()).map_err(|_e| Error::from(ErrorKind::InvalidData))
}

fn parse_header_line(line: String) -> Result<(HeaderName, HeaderValue), Error> {
  let mut bytes = line.split(":");
  match (bytes.next(), bytes.next()) {
    (Some(left), Some(right)) => Ok((parse_header_name(left)?, parse_header_value(right)?)),
    _ => Err(Error::from(ErrorKind::InvalidData)),
  }
}

fn parse_method(raw_value: &str) -> Result<Method, Error> {
  Method::from_bytes(raw_value.as_bytes()).map_err(|_e| Error::from(ErrorKind::InvalidData))
}

fn parse_request_path(raw_value: &str) -> Result<Uri, Error> {
  http::Uri::builder()
    .path_and_query(raw_value)
    .build()
    .map_err(|_| Error::from(ErrorKind::AddrNotAvailable))
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
  headers: HeaderMap,
  method: Method,
  uri: Uri,
}

async fn read_headers<T>(reader: T) -> Result<RequestHead, Error>
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

  let (method, uri) = parse_request_line(request_line)?;
  Ok(RequestHead {
    headers: map,
    method,
    uri,
  })
}

async fn write_response<T, U>(mut writer: T, response: Response<U>) -> Result<(), Error>
where
  T: async_std::io::Write + std::marker::Unpin,
{
  let (bits, _) = response.into_parts();
  let bytes = format!(
    "HTTP/1.0 {} {}\r\n",
    bits.status.as_str(),
    bits.status.canonical_reason().unwrap_or_default(),
  );

  writer
    .write(bytes.as_bytes())
    .await
    .map_err(|_| Error::from(ErrorKind::Other))?;

  let headers = bits
    .headers
    .iter()
    .map(|(key, value)| value.to_str().map(|v| format!("{}: {}", key, v)))
    .flatten()
    .collect::<Vec<String>>()
    .join("\r\n");

  let out = format!("{}\r\n", headers);

  writer
    .write(out.as_bytes())
    .await
    .map_err(|_| Error::from(ErrorKind::Other))?;

  Ok(())
}

fn date() -> Result<HeaderValue, Error> {
  HeaderValue::from_str(
    format!(
      "{}",
      Utc::now()
        .with_timezone(&chrono_tz::GMT)
        .format("%a, %e %b %Y %H:%M:%S GMT")
        .to_string()
    )
    .as_str(),
  )
  .or(Err(Error::from(ErrorKind::InvalidData)))
}

#[derive(Debug, Deserialize)]
struct TokenExchangePayload {
  access_token: String,
}

#[derive(Debug, Deserialize)]
struct UserInfoPayload {
  name: String,
  sub: String,
  email: String,
  picture: String,
}

fn make_client() -> Result<isahc::HttpClient, Error> {
  isahc::HttpClient::new().map_err(|e| {
    Error::new(
      ErrorKind::Other,
      format!("unable to open http connection: {:?}", e),
    )
  })
}

async fn fetch_info(authorization: TokenExchangePayload) -> Result<UserInfoPayload, Error> {
  let client = make_client()?;
  let mut request = Request::builder();
  let bearer = format!("Bearer {}", authorization.access_token);
  request
    .method(Method::GET)
    .uri(constants::GOOGLE_INFO_URL)
    .header("Authorization", bearer.as_str());

  match client.send(
    request
      .body(())
      .map_err(|e| Error::new(ErrorKind::Other, format!("{}", e)))?,
  ) {
    Ok(mut response) if response.status() == 200 => serde_json::from_reader(response.body_mut())
      .map_err(|e| Error::new(ErrorKind::Other, format!("{}", e))),
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

  match client.post(constants::GOOGLE_TOKEN_URL, encoded) {
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

async fn send_redirect<T>(writer: T, location: &str) -> Result<(), Error>
where
  T: async_std::io::Write + std::marker::Unpin,
{
  let mut out = Response::builder();
  out
    .status(StatusCode::FOUND)
    .header(http::header::LOCATION, location);

  if let Ok(value) = date() {
    out.header(http::header::DATE, value);
  }

  match out.body(()) {
    Ok(response) => write_response(writer, response).await,
    Err(e) => {
      println!("[warning] problem building response {:?}", e);
      return Err(Error::from(ErrorKind::NotFound));
    }
  }
}

async fn bad_request<T>(writer: T) -> Result<(), Error>
where
  T: async_std::io::Write + std::marker::Unpin,
{
  let mut out = Response::builder();
  out.status(StatusCode::BAD_REQUEST);

  if let Ok(value) = date() {
    out.header(http::header::DATE, value);
  }

  match out.body(()) {
    Ok(response) => write_response(writer, response).await,
    Err(e) => {
      println!("[warning] problem building response {:?}", e);
      return Err(Error::from(ErrorKind::NotFound));
    }
  }
}

async fn not_found<T>(writer: T) -> Result<(), Error>
where
  T: async_std::io::Write + std::marker::Unpin,
{
  let mut out = Response::builder();
  out.status(StatusCode::NOT_FOUND);

  if let Ok(value) = date() {
    out.header(http::header::DATE, value);
  }

  match out.body(()) {
    Ok(response) => write_response(writer, response).await,
    Err(e) => {
      println!("[warning] problem building response {:?}", e);
      return Err(Error::from(ErrorKind::NotFound));
    }
  }
}

async fn authenticate<T>(writer: T, uri: Uri, config: &Configuration) -> Result<(), Error>
where
  T: async_std::io::Write + std::marker::Unpin,
{
  let code = match form_urlencoded::parse(uri.query().unwrap_or_default().as_bytes())
    .find(|(key, _)| key == "code")
  {
    Some((_, code)) => code,
    None => return not_found(writer).await,
  };

  let authorization = match exchange_code(&code, config).await {
    Ok(token) => token,
    Err(e) => {
      println!("[warning] unable to exchange code: {:?}", e);
      return bad_request(writer).await;
    }
  };

  let user_info = fetch_info(authorization).await.map_err(|e| {
    Error::new(
      ErrorKind::Other,
      format!("unable to loader user info: {:?}", e),
    )
  })?;

  let redis_client =
    redis::Client::open(config.krumi.session_store.redis_uri.as_str()).map_err(|e| {
      Error::new(
        ErrorKind::Other,
        format!("invalid redis configuration url: {}", e),
      )
    })?;

  let mut con = redis_client.get_connection().map_err(|e| {
    Error::new(
      ErrorKind::Other,
      format!("unable to open connection: {}", e),
    )
  })?;

  redis::cmd("PING")
    .query(&mut con)
    .map_err(|e| Error::new(ErrorKind::Other, format!("unable to ping server: {}", e)))?;

  println!("[debug] successully loaded and saved user: {:?}", user_info);

  send_redirect(writer, config.krumi.auth_uri.as_str()).await
}

async fn identify<T>(writer: T) -> Result<(), Error>
where
  T: async_std::io::Write + std::marker::Unpin,
{
  not_found(writer).await
}

async fn login<T>(writer: T, config: &Configuration) -> Result<(), Error>
where
  T: async_std::io::Write + std::marker::Unpin,
{
  println!("[debug] login attempt, building redir");

  let mut location = Url::parse(GOOGLE_AUTH_URL).map_err(|_| Error::from(ErrorKind::Other))?;
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

  send_redirect(writer, location.as_str()).await
}

async fn handle<T>(mut stream: TcpStream, config: T) -> Result<(), Error>
where
  T: std::convert::AsRef<Configuration>,
{
  let headers = match read_headers(&stream).await {
    Ok(v) => v,
    Err(e) => {
      println!("[warning] unable to parse headers: {:?}", e);
      return Err(e);
    }
  };

  match (headers.method, headers.uri.path()) {
    (Method::GET, "/auth/redirect") => login(&mut stream, config.as_ref()).await?,
    (Method::GET, "/auth/callback") => {
      authenticate(&mut stream, headers.uri, config.as_ref()).await?
    }
    (Method::GET, "/auth/identify") => identify(&stream).await?,
    _ => {
      println!("[debug] 404 for {:?}", headers.uri);
      not_found(&mut stream).await?;
    }
  }

  stream.flush().await
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
  let broker = task::spawn(broker_loop(receiver));
  let shared_config = Arc::from(configuration.clone());

  while let Some(stream) = incoming.next().await {
    match stream {
      Ok(connection) => {
        let local_config = shared_config.clone();
        task::spawn(handle(connection, local_config));
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
