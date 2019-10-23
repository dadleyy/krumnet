extern crate async_std;
extern crate http;
extern crate url;

pub mod configuration;
pub mod constants;

use async_std::io::BufReader;
use async_std::net::{TcpListener, TcpStream};
use async_std::prelude::*;
use async_std::task;
use configuration::Configuration;
use constants::GOOGLE_AUTH_URL;
use http::header::{HeaderMap, HeaderName, HeaderValue};
use http::{Method, Uri};
use std::io::{Error, ErrorKind};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use url::Url;

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
  println!("[debug] parsing request path: {}", raw_value);
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
  path: Uri,
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

  println!("[debug] starting header parse for {:?}", request_line);

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

  let (method, path) = parse_request_line(request_line)?;
  Ok(RequestHead {
    headers: map,
    method,
    path,
  })
}

async fn not_found<T>(mut writer: T) -> Result<(), Error>
where
  T: async_std::io::Write + std::marker::Unpin,
{
  writer
    .write(
      b"HTTP/1.0 404 Not Found\r\nContent-Length: 9\r\nContent-Type: text/plain\r\n\r\nnot found",
    )
    .await?;
  Ok(())
}

async fn authenticate<T>(mut writer: T, config: &Configuration) -> Result<(), Error>
where
  T: async_std::io::Write + std::marker::Unpin,
{
  writer
    .write(b"HTTP/1.0 200 Ok\r\nContent-Length: 2\r\nContent-Type: text/plain\r\n\r\nok")
    .await?;
  Ok(())
}

async fn login<T>(mut writer: T, config: &Configuration) -> Result<(), Error>
where
  T: async_std::io::Write + std::marker::Unpin,
{
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
  println!("[debug] request processed: {:?}", headers);

  match (headers.method, headers.path.path()) {
    (Method::GET, "/auth/redirect") => login(&mut stream, config.as_ref()).await?,
    (Method::GET, "/auth/callback") => authenticate(&mut stream, config.as_ref()).await?,
    _ => {
      println!("[debug] 404 for {:?}", headers.path);
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

pub async fn run(
  addr: String,
  configuration: Configuration,
) -> Result<(), Box<dyn std::error::Error>> {
  let listener = TcpListener::bind(addr).await?;
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
