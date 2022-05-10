use std::io;
use std::io::prelude::*;
use std::net::TcpStream;
use std::sync::mpsc;
use std::thread;
use aes::Aes128;
use aes::cipher::{
    BlockEncrypt, BlockDecrypt, KeyInit, generic_array::GenericArray
};

extern crate postgres;

use postgres::{Client, NoTls, Error};

const DATABASE: &str = "postgresql://postgres:123@localhost/test";


// structure to read from database
struct ChatMessage {
    _id: i32,
    ip: String,
    msg: Option<Vec<u8>>
}

fn encrypt_msg(buffer: Vec<u8>, cipher: &Aes128) -> Vec<u8> {
    let blocks: Vec<_> = buffer.chunks(16).collect();

    let mut enc_buffer = Vec::<[u8;16]>::new();

    for b in blocks {
        let b: [u8;16] = b.try_into().unwrap();
        let mut enc = GenericArray::from(b);
        cipher.encrypt_block(&mut enc);
        let enc: [u8;16] = enc.try_into().unwrap();
        enc_buffer.push(enc);
    }

    enc_buffer.concat()
}

fn decrypt_msg(buffer: Vec<u8>, cipher: &Aes128) -> Vec<u8> {
    let blocks: Vec<_> = buffer.chunks(16).collect();

    let mut dec_buffer = Vec::<[u8;16]>::new();

    for b in blocks {
        let b: [u8;16] = b.try_into().unwrap();
        let mut dec = GenericArray::from(b);
        cipher.decrypt_block(&mut dec);
        let dec: [u8;16] = dec.try_into().unwrap();
        dec_buffer.push(dec);
    }

    dec_buffer.concat()
}

fn handle_connection(stream: &mut TcpStream, rx: std::sync::mpsc::Receiver<std::string::String>, cipher:Aes128) {
    loop {
        let mut buffer = [0; 1024];
        match stream.read(&mut buffer) {
            Ok(_) => {
                // decrypt read buffer
                let dec_buffer = decrypt_msg(Vec::from(buffer), &cipher);
                assert_eq!(Vec::from(buffer), encrypt_msg(dec_buffer.clone(), &cipher));

                let msg = String::from_utf8_lossy(&dec_buffer).to_string();
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
                // encrypt msg to send
                let buffer = msg.clone().into_bytes();
                let mut blocks = [0u8; 1024];
                blocks[..buffer.len()].clone_from_slice(&buffer);

                let enc_buffer = encrypt_msg(Vec::from(blocks), &cipher);
                assert_eq!(Vec::from(blocks), decrypt_msg(enc_buffer.clone(), &cipher));

                stream.write(&enc_buffer).unwrap();
            },
            Err(_) => {}
        }
    }
}


fn main() {
    let mut stream = TcpStream::connect("127.0.0.1:7878").unwrap();
    stream.set_nonblocking(true).unwrap();

    let (tx, rx) = mpsc::channel::<String>();

    let key = GenericArray::from([0u8; 16]);
    let cipher = Aes128::new(&key);
    
    thread::spawn(move || {
        handle_connection(&mut stream, rx, Aes128::new(&key));
    });

    println!("type 'send ...' to send message\ntype 'history' to see chat history");
    loop {
        // read command
        let mut cmd = String::new();
        io::stdin()
            .read_line(&mut cmd)
            .expect("Fail");

        if cmd.starts_with("send ") {
            let msg = String::from(&cmd[5..cmd.chars().count()]);
            tx.send(msg).unwrap();
        }
        else {
            if cmd == String::from("history\r\n") {
                //get messages from database
                let mut sqlclient = Client::connect(DATABASE, NoTls).unwrap();
                for row in sqlclient.query("SELECT id, ip, msg FROM chatmessage", &[]).unwrap() {
                    let chatmessage = ChatMessage {
                        _id: row.get(0),
                        ip: row.get(1),
                        msg: row.get(2),
                    };

                    println!("id={} ip={}: {}",
                            chatmessage._id,
                            chatmessage.ip,
                            String::from_utf8_lossy(&decrypt_msg(chatmessage.msg.unwrap(), &cipher)).to_string());
                }
            }
            else {
                println!("wrong command");
            }
        }
    }
}
