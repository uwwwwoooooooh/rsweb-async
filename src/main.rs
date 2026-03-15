use std::{
    fs,
    time::Duration,
};
use async_std::{
    task,
    io::{BufReader, prelude::*},
    net::TcpListener,
};
use futures::{AsyncRead, AsyncWrite};
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

async fn handle_connection<S>(mut stream: S)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let buf_reader = BufReader::new(&mut stream);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::pin::Pin;

    use futures::{
        io::Error,
        {AsyncRead, AsyncWrite},
        task::{Context, Poll},
    };

    struct MockTcpStream {
        input: Vec<u8>,
        output: Vec<u8>,
        read_pos: usize,
    }

    impl MockTcpStream {
        fn new(input: Vec<u8>) -> Self {
            Self {
                input,
                output: Vec::new(),
                read_pos: 0,
            }
        }

        fn get_output(&self) -> &[u8] {
            &self.output
        }
    }

    impl AsyncRead for MockTcpStream {
        fn poll_read(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<Result<usize, Error>> {
            let this = self.get_mut();
            if this.read_pos >= this.input.len() {
                return Poll::Ready(Ok(0));
            }
            let to_read = std::cmp::min(buf.len(), this.input.len() - this.read_pos);
            buf[..to_read].copy_from_slice(&this.input[this.read_pos..this.read_pos + to_read]);
            this.read_pos += to_read;
            Poll::Ready(Ok(to_read))
        }
    }

    impl AsyncWrite for MockTcpStream {
        fn poll_write(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<Result<usize, Error>> {
            let this = self.get_mut();
            this.output.extend_from_slice(buf);
            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
            Poll::Ready(Ok(()))
        }

        fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
            Poll::Ready(Ok(()))
        }
    }

    impl Unpin for MockTcpStream {}

    #[async_std::test]
    async fn test_handle_connection() {
        let request = b"GET / HTTP/1.1\r\n\r\n";
        let mut mock = MockTcpStream::new(request.to_vec());
        handle_connection(&mut mock).await;
        let output = mock.get_output();
        assert!(output.starts_with(b"HTTP/1.1 200 OK"));
    }
}