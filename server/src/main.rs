use std::io;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use std::thread;
use std::sync::mpsc;
use aes::Aes128;
use aes::cipher::{
    BlockCipher, BlockEncrypt, BlockDecrypt, KeyInit, generic_array::GenericArray
};


fn handle_connection(mut stream: TcpStream, tx: std::sync::mpsc::Sender<(std::string::String, std::net::TcpStream)>) {
    println!("---");
    loop
    {
        let mut buffer = [0; 1024];
        match stream.read(&mut buffer) {
            Ok(_) => {
                let msg = String::from_utf8_lossy(&buffer).to_string();
                println!("{}: {}", stream.peer_addr().unwrap(), msg);
                println!("+++");
                tx.send((msg, stream.try_clone().unwrap())).unwrap();
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

    let mut clients: Vec<TcpStream> = Vec::new();
    let (tx, rx) = mpsc::channel::<(String, TcpStream)>();

    loop {
        if let Ok((stream, addr)) = listener.accept() {
            println!("connected {}", stream.peer_addr().unwrap());

            clients.push(stream.try_clone().unwrap());
            let tx = tx.clone();

            thread::spawn(move ||{
                handle_connection(stream, tx);
            });
        }
        

        match rx.try_recv() {
            Ok((msg, sender_stream)) => {
                let clients_to_send_msg = (&clients).into_iter().filter(|c| c.peer_addr().unwrap() != sender_stream.peer_addr().unwrap()).collect::<Vec<_>>();
                for mut c in clients_to_send_msg.into_iter() {
                    let mut buff = msg.clone().into_bytes();
                    c.write(&buff).unwrap();
                }
            },
            Err(e) => {}
        }
    }
}

