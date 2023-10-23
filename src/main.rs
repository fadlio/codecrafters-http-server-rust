use std::io::{BufRead, Read, Write};
use std::net::{TcpListener, TcpStream};
use anyhow::{anyhow, Result};
use itertools::Itertools;

enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
}

impl From<&str> for HttpMethod {
    fn from(value: &str) -> Self {
        match value {
            "GET" => HttpMethod::GET,
            _ => unimplemented!()
        }
    }
}

struct HttpFrame<'a> {
    method: HttpMethod,
    path: &'a str,
    version: &'a str,
}

impl HttpFrame<'_> {
    fn from_request_str(buffer: &str) -> Result<HttpFrame> {
        let header = buffer.split("\r\n").next().ok_or(anyhow!("Invalid frame"))?;
        match header.split(' ').collect_tuple() {
            Some((method, path, version)) => Ok(HttpFrame {
                method: HttpMethod::from(method),
                version,
                path,
            }),
            _ => Err(anyhow!("Invalid frame"))
        }
    }
}


fn handle_connection(mut stream: TcpStream) -> Result<()> {
    let mut buffer: [u8; 128] = [0; 128];
    stream.read(&mut buffer)?;
    let request = std::str::from_utf8(&buffer)?;
    let frame = HttpFrame::from_request_str(&request)?;
    match frame.path {
        "/" => stream.write(b"HTTP/1.1 200 OK\r\n\r\n")?,
        _ => stream.write(b"HTTP/1.1 404 Not Found\r\n\r\n")?,
    };

    Ok(())
}

fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:4221")?;

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
