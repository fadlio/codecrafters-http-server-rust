use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use anyhow::Result;

fn handle_connection(mut stream: TcpStream) -> Result<()> {
    let mut buffer: [u8; 128] = [0; 128];
    stream.read(&mut buffer)?;
    stream.write(b"HTTP/1.1 200 OK\r\n\r\n")?;
    Ok(())
}

fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("accepted new connection");
                handle_connection(stream)?;
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
    Ok(())
}
