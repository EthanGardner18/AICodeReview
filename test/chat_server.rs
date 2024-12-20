// chat_server.rs
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::thread;

fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0; 512];
    loop {
        let bytes_read = match stream.read(&mut buffer) {
            Ok(0) => return,
            Ok(n) => n,
            Err(_) => {
                eprintln!("Error reading from stream");
                return;
            }
        };

        if stream.write(&buffer[0..bytes_read]).is_err() {
            eprintln!("Error writing to stream");
            return;
        }
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || {
                    handle_client(stream);
                });
            }
            Err(_) => eprintln!("Failed to accept a connection"),
        }
    }
}

