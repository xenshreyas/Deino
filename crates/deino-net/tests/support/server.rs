//! A scripted HTTP/1.1 server on a local socket, for downloader tests.
//!
//! Each connection gets the next scripted [`Reply`] and is closed after it.
//! A reply can stall: it promises its whole body, sends part of it, and then
//! holds the connection open without sending more.

#![allow(
    dead_code,
    reason = "shared by several test crates (`#[path]`), each using part of it"
)]

use std::fmt::Write as _;
use std::sync::{Arc, Mutex, PoisonError};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinSet;

/// One scripted response.
#[derive(Clone, Debug)]
pub struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    stall_after: Option<usize>,
}

impl Reply {
    /// `status` with `body`.
    pub fn new(status: u16, body: &[u8]) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: body.to_vec(),
            stall_after: None,
        }
    }

    /// Adds a header.
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_owned(), value.to_owned()));
        self
    }

    /// Sends only the first `bytes` of the body, then goes silent.
    pub fn stall_after(mut self, bytes: usize) -> Self {
        self.stall_after = Some(bytes);
        self
    }

    fn head(&self) -> String {
        let mut head = format!(
            "HTTP/1.1 {} Scripted\r\ncontent-length: {}\r\nconnection: close\r\n",
            self.status,
            self.body.len()
        );
        for (name, value) in &self.headers {
            write!(head, "{name}: {value}\r\n").unwrap();
        }
        head + "\r\n"
    }
}

/// The running server. Dropping it stops it.
pub struct Server {
    /// `http://127.0.0.1:<port>`.
    pub base: String,
    requests: Arc<Mutex<Vec<String>>>,
    _tasks: JoinSet<()>,
}

impl Server {
    /// Serves `replies`, one per connection, in order.
    pub async fn start(replies: Vec<Reply>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let mut tasks = JoinSet::new();
        tasks.spawn(serve(listener, replies, requests.clone()));
        Self {
            base,
            requests,
            _tasks: tasks,
        }
    }

    /// The request heads received so far, lower-cased.
    pub fn requests(&self) -> Vec<String> {
        self.requests
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

async fn serve(listener: TcpListener, replies: Vec<Reply>, requests: Arc<Mutex<Vec<String>>>) {
    let mut connections = JoinSet::new();
    for reply in replies {
        let (socket, _) = listener.accept().await.unwrap();
        connections.spawn(answer(socket, reply, requests.clone()));
    }
    while connections.join_next().await.is_some() {}
}

async fn answer(mut socket: TcpStream, reply: Reply, requests: Arc<Mutex<Vec<String>>>) {
    let head = read_head(&mut socket).await;
    requests
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(head.to_ascii_lowercase());
    socket.write_all(reply.head().as_bytes()).await.unwrap();
    let sent = reply.stall_after.unwrap_or(reply.body.len());
    socket.write_all(&reply.body[..sent]).await.unwrap();
    socket.flush().await.unwrap();
    if reply.stall_after.is_some() {
        std::future::pending::<()>().await;
    }
}

async fn read_head(socket: &mut TcpStream) -> String {
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        if socket.read(&mut byte).await.unwrap() == 0 {
            break;
        }
        head.push(byte[0]);
    }
    String::from_utf8_lossy(&head).into_owned()
}
