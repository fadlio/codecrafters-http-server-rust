use std::collections::HashMap;
use std::io::{BufRead, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use anyhow::{anyhow, Context, Result};
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

struct HttpRequest<'a> {
    method: HttpMethod,
    path: &'a str,
    version: &'a str,
    headers: HashMap<String, String>,
}

impl HttpRequest<'_> {
    fn from_request_str(buffer: &str) -> Result<HttpRequest> {
        let mut lines = buffer.split("\r\n");

        let (method, path, version) = lines
            .next().ok_or(anyhow!("Invalid frame"))?
            .split(' ')
            .collect_tuple().ok_or(anyhow!("Invalid frame"))?;

        let headers: HashMap<_, _> = lines
            .filter_map(|l| {
                if let Some((key, value)) = l.split_once(": ") {
                    Some((
                        key.to_string(),
                        value.to_string(),
                    ))
                } else {
                    None
                }
            })
            .collect();

        Ok(HttpRequest {
            method: HttpMethod::from(method),
            version,
            path,
            headers,
        })
    }
}

fn send_text_plain(mut stream: &TcpStream, text: &str) -> Result<usize> {
    let mut data = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n".to_string();
    data.push_str(&*format!("Content-Length: {}\r\n\r\n", text.len()));
    data.push_str(text);
    let bytes_written = stream.write(data.as_bytes()).context("Failed to send")?;
    Ok(bytes_written)
}

fn not_found_route(mut stream: &TcpStream) -> Result<usize> {
    stream.write(b"HTTP/1.1 404 Not Found\r\n\r\n").context("Failed to send")
}

fn index_route(mut stream: &TcpStream) -> Result<usize> {
    stream.write(b"HTTP/1.1 200 OK\r\n\r\n").context("Failed to send")
}

fn echo_route(stream: &TcpStream, frame: &HttpRequest) -> Result<usize> {
    send_text_plain(&stream, &frame.path[6..])
}

fn user_agent_route(stream: &TcpStream, frame: &HttpRequest) -> Result<usize> {
    let user_agent = frame.headers.get("User-Agent")
        .ok_or(anyhow!("User Agent not found"))?;
    send_text_plain(&stream, user_agent.as_str())
}


fn route_connection(mut stream: TcpStream) -> Result<()> {
    let mut buffer: [u8; 512] = [0; 512];
    stream.read(&mut buffer)?;
    let request = std::str::from_utf8(&buffer)?;
    let frame = HttpRequest::from_request_str(&request)?;
    match frame.path {
        "/" => index_route(&stream),
        _ if frame.path.starts_with("/echo/") => echo_route(&stream, &frame),
        _ if frame.path.starts_with("/user-agent") => user_agent_route(&stream, &frame),
        _ => not_found_route(&stream),
    }?;

    Ok(())
}

fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:4221")?;
    // let pool = ThreadPool::new(4);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("accepted new connection");
                thread::spawn(|| {
                    route_connection(stream).unwrap();
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
    Ok(())
}
