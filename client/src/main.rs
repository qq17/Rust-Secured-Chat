use std::io;
use std::io::prelude::*;
use std::net::TcpStream;
use std::sync::mpsc;
use std::thread;
use aes::Aes128;
use aes::cipher::{
    BlockCipher, BlockEncrypt, BlockDecrypt, KeyInit, generic_array::GenericArray
};


fn handle_connection(mut stream: &mut TcpStream, rx: std::sync::mpsc::Receiver<std::string::String>) {
    let key = GenericArray::from([0u8; 16]);
    let cipher = Aes128::new(&key);

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
                let mut buff: Vec<_> = buff.chunks(16).collect();
                
                //Doesnt work

                // let mut blocks = GenericArray::from_iter(buff);
                let a:[[u8;16];64] = buff.into_iter().collect().try_into().unwrap();
                let mut blocks = GenericArray::from(a);

                cipher.encrypt_blocks(&mut blocks);
                let mut buf: Vec<u8> = blocks.concat();
                let mut msg = String::from_utf8_lossy(&buf).to_string();

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
