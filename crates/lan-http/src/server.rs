//! One thread, one request at a time, over `tiny_http`.
//!
//! The listener knows about framing and caps, not about routes, tokens or JSON.
//! The caller hands in a handler and decides what a path means; this module
//! only turns the bytes into an [`Incoming`] and writes the [`Reply`] back.
//!
//! Synchronous on purpose: a shop has a handful of phones and two or three
//! tills. An async runtime would be more machinery than the problem needs.

use std::io::Read;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};

use tiny_http::{Header, Request, Response, Server as TinyServer};

use crate::Error;

/// One request, read completely, with header names lower-cased.
pub struct Incoming {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Incoming {
    /// The first header with this name, case-insensitively.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(field, _)| field.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

/// What the handler wants written back.
pub struct Reply {
    pub status: u16,
    pub content_type: &'static str,
    pub body: Vec<u8>,
}

impl Reply {
    pub fn json(status: u16, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            content_type: "application/json; charset=utf-8",
            body: body.into(),
        }
    }

    pub fn text(status: u16, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            content_type: "text/plain; charset=utf-8",
            body: body.into(),
        }
    }
}

/// A bound listener with one worker thread.
///
/// Dropping the server stops it: the stop flag is set, `unblock` wakes a worker
/// parked in `recv`, and the thread is joined.
pub struct Server {
    inner: Arc<TinyServer>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    port: u16,
}

impl Server {
    /// Bind `0.0.0.0:port` and start serving. `port = 0` asks the OS for a
    /// free one; read it back with [`Server::port`].
    ///
    /// `max_body` is the largest request body the server will read. The handler
    /// runs after the body is in memory, so a smaller per-route cap is a policy
    /// check inside the handler, not a second read.
    pub fn bind<H>(port: u16, max_body: usize, handler: H) -> Result<Self, Error>
    where
        H: FnMut(Incoming) -> Reply + Send + 'static,
    {
        let inner = Arc::new(
            TinyServer::http(("0.0.0.0", port)).map_err(|error| Error::Bind(error.to_string()))?,
        );
        let bound = inner
            .server_addr()
            .to_ip()
            .map(|address| address.port())
            .unwrap_or(port);
        let stop = Arc::new(AtomicBool::new(false));

        let worker = {
            let inner = Arc::clone(&inner);
            let stop = Arc::clone(&stop);
            thread::Builder::new()
                .name("lan-http".to_owned())
                .spawn(move || run(inner, max_body, handler, stop))
                .map_err(|error| Error::Bind(error.to_string()))?
        };

        Ok(Self {
            inner,
            stop,
            worker: Some(worker),
            port: bound,
        })
    }

    /// The port actually bound, which matters when `0` was requested.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Stop and wait for the worker. Idempotent with `Drop`.
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        // Wake a worker blocked in recv() so it observes `stop` and exits
        // instead of waiting for a request that may never come.
        self.inner.unblock();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn run<H>(server: Arc<TinyServer>, max_body: usize, mut handler: H, stop: Arc<AtomicBool>)
where
    H: FnMut(Incoming) -> Reply,
{
    while !stop.load(Ordering::Relaxed) {
        match server.recv() {
            Ok(request) => handle(request, max_body, &mut handler),
            Err(_) if stop.load(Ordering::Relaxed) => break,
            // `recv()` only fails when the accept socket is broken. Ending the
            // loop is honest: the caller sees no more responses and can restart
            // the listener. Spinning on a dead socket would not help.
            Err(_) => break,
        }
    }
}

fn handle<H>(mut request: Request, max_body: usize, handler: &mut H)
where
    H: FnMut(Incoming) -> Reply,
{
    let method = request.method().as_str().to_owned();
    let path = request
        .url()
        .split('?')
        .next()
        .unwrap_or_default()
        .to_owned();

    // We only speak HTTP/1.1 with a declared length. A chunked request is
    // rejected rather than half-read: there is no producer in the protocol
    // that needs it, and it removes a length ambiguity.
    let has_body = matches!(method.as_str(), "POST" | "PUT" | "PATCH");
    if request
        .headers()
        .iter()
        .any(|header| header.field.equiv("transfer-encoding"))
    {
        return reply(
            request,
            Reply::json(411, r#"{"ok":false,"error":"length required"}"#),
        );
    }
    if has_body && request.body_length().is_none() {
        return reply(
            request,
            Reply::json(411, r#"{"ok":false,"error":"length required"}"#),
        );
    }
    if request
        .body_length()
        .is_some_and(|length| length > max_body)
    {
        return reply(
            request,
            Reply::json(413, r#"{"ok":false,"error":"body too large"}"#),
        );
    }

    // Read one byte past the cap so a missing or lying length cannot become an
    // unbounded allocation.
    let mut body = Vec::new();
    let read = {
        let mut limited = request.as_reader().take((max_body + 1) as u64);
        limited.read_to_end(&mut body)
    };
    if read.is_err() {
        return reply(
            request,
            Reply::json(400, r#"{"ok":false,"error":"could not read body"}"#),
        );
    }
    if body.len() > max_body {
        return reply(
            request,
            Reply::json(413, r#"{"ok":false,"error":"body too large"}"#),
        );
    }

    let headers = request
        .headers()
        .iter()
        .map(|header| {
            (
                header.field.as_str().as_str().to_ascii_lowercase(),
                header.value.as_str().to_owned(),
            )
        })
        .collect();

    let incoming = Incoming {
        method,
        path,
        headers,
        body,
    };
    let answer = handler(incoming);
    reply(request, answer);
}

fn reply(request: Request, answer: Reply) {
    let mut response = Response::from_data(answer.body).with_status_code(answer.status);
    if let Ok(header) = Header::from_bytes("Content-Type", answer.content_type) {
        response.add_header(header);
    }
    if let Ok(header) = Header::from_bytes("Cache-Control", "no-store") {
        response.add_header(header);
    }
    if let Ok(header) = Header::from_bytes("Connection", "close") {
        response.add_header(header);
    }
    let _ = request.respond(response);
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    use super::*;

    fn start<H>(max_body: usize, handler: H) -> Server
    where
        H: FnMut(Incoming) -> Reply + Send + 'static,
    {
        Server::bind(0, max_body, handler).expect("bind")
    }

    fn request(port: u16, raw: &str) -> (u16, String) {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("timeout");
        stream.write_all(raw.as_bytes()).expect("write");
        let mut response = String::new();
        stream.read_to_string(&mut response).expect("read");
        let status = response
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|code| code.parse().ok())
            .expect("status");
        (status, response)
    }

    #[test]
    fn the_handler_sees_method_path_header_and_body() {
        let server = start(1024, |incoming| {
            let token = incoming.header("x-till-token").unwrap_or("none");
            Reply::json(
                200,
                format!(
                    "{} {} {token} {}",
                    incoming.method,
                    incoming.path,
                    String::from_utf8_lossy(&incoming.body)
                ),
            )
        });
        let (status, response) = request(
            server.port(),
            "POST /v1/echo?ignored=1 HTTP/1.1\r\nHost: x\r\nX-Till-Token: 7QK4M2X9AB\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello",
        );
        assert_eq!(status, 200);
        assert!(
            response.ends_with("POST /v1/echo 7QK4M2X9AB hello"),
            "{response}"
        );
    }

    #[test]
    fn a_get_without_a_body_is_allowed() {
        let server = start(1024, |incoming| Reply::json(200, incoming.method));
        let (status, response) = request(
            server.port(),
            "GET /v1/ping HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n",
        );
        assert_eq!(status, 200);
        assert!(response.ends_with("GET"), "{response}");
    }

    #[test]
    fn a_declared_body_over_the_cap_is_refused_before_reading() {
        let server = start(4, |_| Reply::json(200, "read"));
        let (status, _) = request(
            server.port(),
            "POST /v1/scan HTTP/1.1\r\nHost: x\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello",
        );
        assert_eq!(status, 413);
    }

    #[test]
    fn a_post_without_content_length_is_refused() {
        let server = start(1024, |_| Reply::json(200, "read"));
        let (status, _) = request(
            server.port(),
            "POST /v1/scan HTTP/1.1\r\nHost: x\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
        );
        assert_eq!(status, 411);
    }

    #[test]
    fn the_bound_port_is_reported_and_stopping_unblocks_the_worker() {
        let server = start(64, |_| Reply::json(204, Vec::new()));
        let port = server.port();
        assert_ne!(port, 0);
        server.stop();
        // tiny_http's accept thread notices the close flag asynchronously, so
        // give it a moment to release the socket.
        let refused = (0..100).any(|_| {
            if TcpStream::connect(("127.0.0.1", port)).is_err() {
                true
            } else {
                std::thread::sleep(Duration::from_millis(10));
                false
            }
        });
        assert!(refused, "listener still accepts on port {port}");
    }
}
