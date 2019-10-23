extern crate async_std;
extern crate chrono;
extern crate chrono_tz;
extern crate http;
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
use http::{Method, Response, StatusCode, Uri};
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

async fn authenticate<T>(mut writer: T, uri: Uri) -> Result<(), Error>
where
  T: async_std::io::Write + std::marker::Unpin,
{
  let code = match form_urlencoded::parse(uri.query().unwrap_or_default().as_bytes())
    .find(|(key, _)| key == "code")
  {
    Some((_, code)) => code,
    None => return not_found(writer).await,
  };

  println!("[debug] auth callback w/ code: {:?}", code);
  writer
    .write(b"HTTP/1.0 200 Ok\r\nContent-Length: 2\r\nContent-Type: text/plain\r\n\r\nok")
    .await?;

  Ok(())
}

async fn login<T>(mut writer: T, config: &Configuration) -> Result<(), Error>
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
  let response = format!(
    "HTTP/1.0 302 Found\r\nContent-Length: 0\r\nLocation: {}\r\n\r\n",
    location.as_str()
  );

  writer.write(response.as_bytes()).await?;
  Ok(())
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
    (Method::GET, "/auth/callback") => authenticate(&mut stream, headers.uri).await?,
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
