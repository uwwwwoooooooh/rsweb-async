use std::{
    fs,
    time::Duration,
};
use async_std::{
    task,
    io::{BufReader, prelude::*},
    net::{TcpListener, TcpStream},
};
use futures::stream::StreamExt;

#[async_std::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").await.unwrap();

    // for stream in listener.incoming() {
    //     let stream = stream.unwrap();

    //     handle_connection(stream).await;
    // }

    listener
        .incoming()
        .for_each_concurrent(None, |stream| async {
            let stream = stream.unwrap();
            handle_connection(stream).await;
        })
        .await;
}

async fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&stream);
    let request_line = buf_reader.lines().next().await.unwrap().unwrap();

    let (status_line, filename) = if request_line == "GET / HTTP/1.1" {
        ("HTTP/1.1 200 OK", "hello.html")
    } else if request_line == "GET /slow HTTP/1.1" {
        task::sleep(Duration::from_secs(5)).await;
        ("HTTP/1.1 200 OK", "hello.html")
    } else {
        ("HTTP/1.1 404 NOT FOUND", "404.html")
    };

    let contents = fs::read_to_string(filename).unwrap();
    let length = contents.len();

    let response =
        format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

    stream.write_all(response.as_bytes()).await.unwrap();
}