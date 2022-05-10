use std::io;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use std::thread;
use std::sync::mpsc;

extern crate postgres;

use postgres::{Client, NoTls, Error};

const DATABASE: &str = "postgresql://postgres:123@localhost/test";

fn handle_connection(mut stream: TcpStream, tx: std::sync::mpsc::Sender<([u8;1024], std::net::TcpStream)>) {
    println!("---");
    loop
    {
        let mut buffer = [0; 1024];

        match stream.read(&mut buffer) {
            Ok(_) => {
                let msg = String::from_utf8_lossy(&buffer).to_string();
                println!("{}: {}", stream.peer_addr().unwrap(), msg);

                // send to other clients
                tx.send((buffer, stream.try_clone().unwrap())).unwrap();

                let mut sqlclient = Client::connect(DATABASE, NoTls).unwrap();
                
                // insert into db
                sqlclient.execute(
                    "INSERT INTO chatmessage (ip, msg) VALUES ($1, $2)",
                    &[&stream.peer_addr().unwrap().to_string(), &(&buffer as &[u8])],
                ).unwrap();
                println!("+++");
            },
            Err(e) => {
                if e.kind() != io::ErrorKind::WouldBlock {
                    break;
                }
            }
        }
        stream.flush().unwrap();
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    listener.set_nonblocking(true).unwrap();

    // vector of all connected clients
    let mut clients: Vec<TcpStream> = Vec::new();

    let (tx, rx) = mpsc::channel::<([u8;1024], TcpStream)>();

    let mut sqlclient = Client::connect(DATABASE, NoTls).unwrap();

    sqlclient.batch_execute("
        DROP TABLE IF EXISTS chatmessage;
        CREATE TABLE IF NOT EXISTS chatmessage (
            id              SERIAL PRIMARY KEY,
            ip              VARCHAR NOT NULL,
            msg             BYTEA NOT NULL
            )
    ").unwrap();

    loop {
        if let Ok((stream, addr)) = listener.accept() {
            println!("connected {}", stream.peer_addr().unwrap());
            
            //add new client to pool
            clients.push(stream.try_clone().unwrap());
            let tx = tx.clone();

            thread::spawn(move ||{
                handle_connection(stream, tx);
            });
        }
        

        match rx.try_recv() {
            Ok((buffer, sender_stream)) => {
                // find clients that are different from the sender and send to them
                let clients_to_send_msg = (&clients).into_iter()
                                                    .filter(|c| c.peer_addr().unwrap() != sender_stream.peer_addr().unwrap())
                                                    .collect::<Vec<_>>();
                for mut c in clients_to_send_msg.into_iter() {
                    c.write(&buffer).unwrap();
                }
            },
            Err(_) => {}
        }
    }
}

