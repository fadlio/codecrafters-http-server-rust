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

fn send_text_plain(mut stream: &TcpStream, text: &str) -> Result<usize> {
    let mut data = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n".to_string();
    data.push_str(&*format!("Content-Length: {}\r\n\r\n", text.len()));
    data.push_str(text);
    let bytes_written = stream.write(data.as_bytes())?;
    Ok(bytes_written)
}


fn handle_connection(mut stream: TcpStream) -> Result<()> {
    let mut buffer: [u8; 512] = [0; 512];
    stream.read(&mut buffer)?;
    let request = std::str::from_utf8(&buffer)?;
    let frame = HttpFrame::from_request_str(&request)?;
    match frame.path {
        "/" => stream.write(b"HTTP/1.1 200 OK\r\n\r\n")?,
        _ if frame.path.starts_with("/echo/") => send_text_plain(&stream, &frame.path[6..])?,
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
