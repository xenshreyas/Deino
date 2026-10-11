//! Just enough HTTP/1.1 over a local socket for tests: a browser `GET` and a
//! server that reads one request.

use std::io;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use url::Url;

/// What a browser does with a redirect: `GET`s `url` on this Mac and
/// returns the whole response as text. `localhost` is dialled as
/// `127.0.0.1`.
pub async fn browser_get(url: &Url) -> io::Result<String> {
    let port = url.port().unwrap_or(80);
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await?;
    let target = match url.query() {
        Some(query) => format!("{}?{query}", url.path()),
        None => url.path().to_owned(),
    };
    let request =
        format!("GET {target} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).await?;
    let mut response = String::new();
    stream.read_to_string(&mut response).await?;
    Ok(response)
}

/// One request a test server received.
#[derive(Clone, Debug, Default)]
pub(super) struct ReceivedRequest {
    /// `POST /token`.
    pub(super) line: String,
    /// The lower-cased header lines.
    pub(super) headers: Vec<String>,
    /// The body.
    pub(super) body: Vec<u8>,
}

impl ReceivedRequest {
    /// The path of the request line.
    pub(super) fn path(&self) -> &str {
        self.line.split(' ').nth(1).unwrap_or_default()
    }

    /// The value of the lower-case header `name`.
    pub(super) fn header(&self, name: &str) -> Option<&str> {
        let prefix = format!("{name}:");
        self.headers
            .iter()
            .find_map(|line| line.strip_prefix(&prefix))
            .map(str::trim)
    }
}

/// Reads one request: its head, then `Content-Length` bytes of body.
pub(super) async fn read_request(stream: &mut TcpStream) -> io::Result<ReceivedRequest> {
    let mut head = Vec::new();
    let mut byte = [0_u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        if stream.read(&mut byte).await? == 0 {
            break;
        }
        head.extend_from_slice(&byte);
    }
    let text = String::from_utf8_lossy(&head).into_owned();
    let mut lines = text.split("\r\n").filter(|line| !line.is_empty());
    let line = lines.next().unwrap_or_default().to_owned();
    let headers: Vec<String> = lines.map(str::to_ascii_lowercase).collect();
    let mut request = ReceivedRequest {
        line,
        headers,
        body: Vec::new(),
    };
    let length = request
        .header("content-length")
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    request.body = vec![0; length];
    stream.read_exact(&mut request.body).await?;
    Ok(request)
}

/// Writes a JSON response with `status` and closes.
pub(super) async fn write_json(stream: &mut TcpStream, status: u16, body: &str) -> io::Result<()> {
    let response = format!(
        "HTTP/1.1 {status} Mock\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await
}
