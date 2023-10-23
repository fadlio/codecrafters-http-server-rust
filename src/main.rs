use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::{env, fs, thread};
use std::path::PathBuf;
use std::sync::Arc;
use anyhow::{anyhow, Context, Result};
use itertools::Itertools;

enum HttpMethod {
    GET,
    POST,
}

impl From<&str> for HttpMethod {
    fn from(value: &str) -> Self {
        match value {
            "GET" => HttpMethod::GET,
            "POST" => HttpMethod::POST,
            _ => unimplemented!()
        }
    }
}

struct HttpRequest<'a> {
    method: HttpMethod,
    path: &'a str,
    _version: &'a str,
    headers: HashMap<&'a str, &'a str>,
    body: Option<&'a [u8]>,
}

impl HttpRequest<'_> {
    fn from_request_str(buffer: &str) -> Result<HttpRequest> {
        let mut lines = buffer.split("\r\n");

        let (method, path, _version) = lines
            .next().ok_or(anyhow!("Invalid frame"))?
            .split(' ')
            .collect_tuple().ok_or(anyhow!("Invalid frame"))?;

        let headers: HashMap<_, _> = lines
            .by_ref()
            .map_while(|l| {
                if let Some((key, value)) = l.split_once(": ") {
                    Some((
                        key,
                        value,
                    ))
                } else {
                    None
                }
            }).collect();

        let body = if let Some(body_data) = lines.next() {
            let data_len: usize = headers.get("Content-Length")
                .ok_or(anyhow!("No content-length specified"))?
                .parse()?;
            Some(&body_data.as_bytes()[0..data_len])
        } else {
            None
        };

        Ok(HttpRequest {
            method: HttpMethod::from(method),
            _version,
            path,
            headers,
            body,
        })
    }
}

fn send_binary(mut stream: &TcpStream, data: &Vec<u8>) -> Result<usize> {
    let mut response = "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\n".to_string();
    response.push_str(&*format!("Content-Length: {}\r\n\r\n", data.len()));
    let mut response_bytes = Vec::from(response.as_bytes());
    response_bytes.extend(data);
    stream.write(&response_bytes).context("Send binary stream write")
}

fn send_text_plain(mut stream: &TcpStream, text: &str) -> Result<usize> {
    let mut data = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n".to_string();
    data.push_str(&*format!("Content-Length: {}\r\n\r\n", text.len()));
    data.push_str(text);
    let bytes_written = stream.write(data.as_bytes()).context("Failed to send")?;
    Ok(bytes_written)
}

fn send_not_found(mut stream: &TcpStream) -> Result<usize> {
    stream.write(b"HTTP/1.1 404 Not Found\r\n\r\n").context("Failed to send")
}

fn send_ok(mut stream: &TcpStream) -> Result<usize> {
    stream.write(b"HTTP/1.1 200 OK\r\n\r\n").context("Failed to send")
}

fn send_created(mut stream: &TcpStream) -> Result<usize> {
    stream.write(b"HTTP/1.1 201 CREATED\r\n\r\n").context("Failed to send")
}

fn get_echo(stream: &TcpStream, frame: &HttpRequest) -> Result<usize> {
    send_text_plain(&stream, &frame.path[6..])
}

fn get_user_agent(stream: &TcpStream, frame: &HttpRequest) -> Result<usize> {
    let user_agent = frame.headers.get("User-Agent")
        .ok_or(anyhow!("User Agent not found"))?;
    send_text_plain(&stream, user_agent)
}

fn get_files(stream: &TcpStream, frame: &HttpRequest, dir: &Option<String>) -> Result<usize> {
    let dir = dir.as_ref().ok_or(anyhow!("Directory not specified"))?;
    let mut path = PathBuf::from(dir);
    let filename = &frame.path[7..];
    path.push(filename);
    if let Ok(data) = fs::read(path).context("File read") {
        send_binary(stream, &data)
    } else {
        send_not_found(stream)
    }
}

fn post_files(stream: &TcpStream, frame: &HttpRequest, dir: &Option<String>) -> Result<usize> {
    let dir = dir.as_ref().ok_or(anyhow!("Directory not specified"))?;
    let data = frame.body.ok_or(anyhow!("No body in request"))?;
    let mut path = PathBuf::from(dir);
    let filename = &frame.path[7..];
    path.push(filename);
    fs::write(path, data)?;
    send_created(stream)
}


fn handle_connection(mut stream: TcpStream, config: Arc<Config>) -> Result<()> {
    let mut buffer: [u8; 512] = [0; 512];
    stream.read(&mut buffer)?;
    let request = std::str::from_utf8(&buffer)?;
    let frame = HttpRequest::from_request_str(&request)?;
    match frame.method {
        HttpMethod::GET => {
            match frame.path {
                "/" => send_ok(&stream),
                _ if frame.path.starts_with("/echo/") => get_echo(&stream, &frame),
                _ if frame.path.starts_with("/user-agent") => get_user_agent(&stream, &frame),
                _ if frame.path.starts_with("/files/") => get_files(&stream, &frame, &config.dir),
                _ => send_not_found(&stream),
            }?;
        }
        HttpMethod::POST => {
            match frame.path {
                _ if frame.path.starts_with("/files/") => post_files(&stream, &frame, &config.dir),
                _ => send_not_found(&stream),
            }?;
        }
    }


    Ok(())
}

#[derive(Debug)]
struct Config {
    dir: Option<String>,
}

fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:4221")?;
    let args: Vec<String> = env::args().collect();
    let dir = if args.len() > 2 && args[1].as_str() == "--directory" {
        Some(args[2].clone())
    } else {
        None
    };

    let config = Arc::new(Config {
        dir
    });

    println!("{config:?}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("accepted new connection");
                let cloned_config = config.clone();
                thread::spawn(move || {
                    handle_connection(stream, cloned_config).unwrap();
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
    Ok(())
}
