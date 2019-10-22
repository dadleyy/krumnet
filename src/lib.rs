extern crate http;

pub mod google;

use async_std::io::BufReader;
use async_std::net::{TcpListener, TcpStream};
use async_std::prelude::*;
use async_std::task;
use google::GoogleCredentials;
use http::header::{HeaderMap, HeaderName, HeaderValue};
use http::Method;
use std::io::{Error, ErrorKind};
use std::sync::mpsc::{channel, Receiver};

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

fn parse_request_line(line: String) -> Result<(Method, String), Error> {
  let mut bytes = line.split_whitespace();
  match (bytes.next(), bytes.next()) {
    (Some(left), Some(right)) => Ok((parse_method(left)?, String::from(right))),
    _ => Err(Error::from(ErrorKind::InvalidData)),
  }
}

#[derive(Debug)]
struct RequestHead {
  headers: HeaderMap,
  method: Method,
  path: String,
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

async fn handle(mut stream: TcpStream) -> Result<(), Error> {
  let headers = read_headers(&stream).await?;
  println!("[debug] request processed: {:?}", headers);
  stream
    .write(b"HTTP/1.1 200 OK\r\nContent-Legnth: 0\r\n\r\n")
    .await?;
  stream.flush().await
}

async fn broker_loop(chan: Receiver<String>) {
  println!("[debug] starting broker event loop");

  for msg in chan.iter() {
    println!("[debug] broker has message: {:?}", msg);
  }
}

pub async fn run(addr: String, creds: GoogleCredentials) -> Result<(), Box<dyn std::error::Error>> {
  let listener = TcpListener::bind(addr).await?;
  let mut incoming = listener.incoming();
  let (sender, receiver) = channel::<String>();
  let broker = task::spawn(broker_loop(receiver));

  println!(
    "[debug] listener bound, entering stream processing: {:?}",
    creds
  );

  while let Some(stream) = incoming.next().await {
    match stream {
      Ok(connection) => {
        task::spawn(handle(connection));
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
