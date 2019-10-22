extern crate http;

pub mod google;

use google::GoogleCredentials;
use http::header::{HeaderName, HeaderValue};
use std::sync::mpsc::{channel, Receiver};
use std::io::{Error, ErrorKind};
use async_std::prelude::*;
use async_std::task;
use async_std::io::BufReader;
use async_std::net::{TcpListener, TcpStream};

fn parse_name(raw_value: &str) -> Result<HeaderName, Error> {
    HeaderName::from_bytes(raw_value.as_bytes()).map_err(|e| {
        println!("[warning] invalid header value {:?}", e);
        Error::from(ErrorKind::InvalidData)
    })
}


fn parse_value(raw_value: &str) -> Result<HeaderValue, Error> {
    HeaderValue::from_bytes(raw_value.as_bytes()).map_err(|e| {
        println!("[warning] invalid header value {:?}", e);
        Error::from(ErrorKind::InvalidData)
    })
}

fn parse_bits(line: String) -> Result<(HeaderName, HeaderValue), Error> {
    let mut bytes = line.split(":");
    match (bytes.next(), bytes.next()) {
        (Some(left), Some(right)) => {
            Ok((parse_name(left)?, parse_value(right)?))
        },
        _ => Err(Error::from(ErrorKind::InvalidData)),
    }
}

async fn handle(mut stream: TcpStream) -> Result<(), Error> {
    let mut reader = BufReader::new(&stream).lines().take(10);
    let request_line = reader.next().await.ok_or(Error::from(ErrorKind::InvalidData))??;
    println!("[debug] starting header parse for {:?}", request_line);

    loop {
        match reader.next().await {
            Some(Ok(line)) if line.is_empty() => break,
            Some(Ok(line)) => {
                match parse_bits(line) {
                    Ok((name, value)) => {
                        println!("name[{}] value[{:?}]", name, value);
                   },
                    _ => {
                        return Err(Error::from(ErrorKind::InvalidData));
                    }
                }
            },
            None => break,
            Some(Err(e)) => {
                println!("[error] unable to parse");
                return Err(e);
            },
        }
    }

    stream.write(b"HTTP/1.1 200 OK\r\nContent-Legnth: 0\r\n\r\n").await?;
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

    println!("[debug] listener bound, entering stream processing: {:?}", creds);

    while let Some(stream) = incoming.next().await {
        match stream {
            Ok(connection) => {
                task::spawn(handle(connection));
            },
            Err(e) => {
                println!("[warning] invalid connection: {:?}", e);
                continue
            }
        }
    }

    drop(sender);
    broker.await;

    Ok(())
}

