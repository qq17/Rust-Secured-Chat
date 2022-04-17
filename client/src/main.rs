use std::io;
use std::io::prelude::*;
use std::net::TcpStream;
use std::sync::mpsc;
use std::thread;


fn handle_connection(mut stream: &mut TcpStream, rx: std::sync::mpsc::Receiver<std::string::String>) {
    loop {
        let mut buffer = [0; 1024];
        match stream.read(&mut buffer) {
            Ok(_) => {
                let msg = String::from_utf8_lossy(&buffer).to_string();
                println!("{}", msg);
            },
            Err(e) => {
                if e.kind() != io::ErrorKind::WouldBlock {
                    break;
                }
            }
        }

        match rx.try_recv() {
            Ok(msg) => {
                let mut buff = msg.clone().into_bytes();
                stream.write(&buff).unwrap();
            },
            Err(e) => {}
        }
    }
}


fn main() {
    let mut stream = TcpStream::connect("127.0.0.1:7878").unwrap();
    stream.set_nonblocking(true).unwrap();

    let (tx, rx) = mpsc::channel::<String>();

    thread::spawn(move || {
        handle_connection(&mut stream, rx);
    });

    println!("type 'send ...' to send message");
    loop {
        let mut cmd = String::new();
        let mut msg = String::new();
        io::stdin()
            .read_line(&mut cmd)
            .expect("Fail");
        if cmd.starts_with("send ") {
            msg = String::from(&cmd[5..cmd.chars().count()]);
            tx.send(msg).unwrap();
        }
        else {
            println!("wrong command");
        }
    }
}
