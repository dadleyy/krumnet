extern crate async_std;

mod google;

use std::sync::mpsc::{channel, Receiver, Sender};
use async_std::prelude::*;
use async_std::task;
use async_std::io::BufReader;
use async_std::net::{TcpListener, TcpStream};
use google::GoogleCredentials;
use std::env::{args_os, var_os};

fn not_empty(rstr: &Result<String, std::io::Error>) -> bool {
    if let Ok(s) = rstr {
        return s.is_empty() == false;
    }

    false
}

async fn handle(mut stream: TcpStream, messages: Sender<String>) -> Result<(), std::io::Error> {
    // println!("[debug] stream: {:?}", stream.peer_addr());
    let reader = BufReader::new(&stream);
    let mut header = reader.lines().take(20).take_while(not_empty);
    let _first = header.next().await;

    stream.write(String::from("HTTP/1.0 200 Ok\r\nContent-Length: 0\r\n\r\n").as_bytes()).await?;
    // println!("[debug] first line: {:?}", first);

    if let Err(e) = messages.send(String::from("hello world")) {
        // println!("[debug] unable to send message: {:?}", e);
        return Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe));
    }

    Ok(())
}

async fn broker_loop(chan: Receiver<String>) {
    println!("[debug] starting broker event loop");

    for msg in chan.iter() {
    println!("[debug] broker has message: {:?}", msg);
    }
}

async fn run(addr: String, creds: GoogleCredentials) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(addr).await?;
    let mut incoming = listener.incoming();
    let (sender, receiver) = channel::<String>();
    let broker = task::spawn(broker_loop(receiver));

    println!("[debug] listener bound, entering stream processing: {:?}", creds);

    while let Some(stream) = incoming.next().await {
        match stream {
            Ok(connection) => {
                // println!("[debug] connection received on ({:?}), sending to channel", connection.peer_addr());
                task::spawn(handle(connection, sender.clone()));
            }
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

fn main() {
    let client_id = var_os("GOOGLE_CLIENT_ID")
        .unwrap_or_default()
        .into_string()
        .unwrap_or_default();
    let client_secret = var_os("GOOGLE_CLIENT_SECRET")
        .unwrap_or_default()
        .into_string()
        .unwrap_or_default();

    let google = GoogleCredentials::new(client_id, client_secret);

    let addr = args_os()
        .skip(1)
        .nth(0)
        .unwrap_or_default()
        .into_string()
        .unwrap_or("0.0.0.0:8080".to_string());

    println!("[debug] starting server '{}': {:?}", addr, google);

    if let Err(e) = task::block_on(run(addr, google)) {
        println!("[error] exiting with error: {:?}", e);
    }
}
