//! One HTTP/1.1 request over a `TcpStream`, then close.
//!
//! The client is deliberately not general: no keep-alive, no redirects, no
//! chunked transfer, no TLS. A till asks a peer for one page and hangs up.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use crate::Error;

/// Where a peer listens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Endpoint {
    pub host: String,
    pub port: u16,
}

impl Endpoint {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
        }
    }
}

/// The only methods stride's LAN protocol speaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
}

impl Method {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
        }
    }
}

/// One request. `token` is the pairing token, when the route needs one.
#[derive(Clone, Debug)]
pub struct Request {
    pub method: Method,
    pub path: String,
    pub token: Option<String>,
    pub body: Vec<u8>,
}

impl Request {
    pub fn get(path: impl Into<String>) -> Self {
        Self {
            method: Method::Get,
            path: path.into(),
            token: None,
            body: Vec::new(),
        }
    }

    pub fn post(path: impl Into<String>, body: impl Into<Vec<u8>>) -> Self {
        Self {
            method: Method::Post,
            path: path.into(),
            token: None,
            body: body.into(),
        }
    }

    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }
}

/// A response that reached us and was inside 2xx.
#[derive(Clone, Debug)]
pub struct Response {
    pub status: u16,
    pub body: Vec<u8>,
}

impl Response {
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

/// A client with fixed timeouts and a fixed response cap.
pub struct Client {
    connect_timeout: Duration,
    io_timeout: Duration,
    max_response: usize,
}

impl Default for Client {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(3),
            io_timeout: Duration::from_secs(10),
            max_response: 4 * 1024 * 1024,
        }
    }
}

impl Client {
    pub fn new() -> Self {
        Self::default()
    }

    /// For tests and for callers that know better than the defaults.
    pub fn with_limits(
        connect_timeout: Duration,
        io_timeout: Duration,
        max_response: usize,
    ) -> Self {
        Self {
            connect_timeout,
            io_timeout,
            max_response,
        }
    }

    /// Send one request and read one response.
    ///
    /// Non-2xx answers are [`Error::Status`], not a successful [`Response`]:
    /// the caller asked for something and did not get it. The body is kept so
    /// the product can show the peer's reason.
    pub fn request(&self, endpoint: &Endpoint, request: &Request) -> Result<Response, Error> {
        if !request.path.starts_with('/')
            || request.path.contains(['\r', '\n'])
            || request.path.len() > 2048
        {
            return Err(Error::Malformed(
                "path must start with /, be short and contain no newlines".to_owned(),
            ));
        }
        if let Some(token) = &request.token
            && !crate::pairing::is_valid(token)
        {
            return Err(Error::Malformed("token is not a pairing code".to_owned()));
        }

        let address = (endpoint.host.as_str(), endpoint.port)
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| {
                Error::Malformed(format!("{} did not resolve to an address", endpoint.host))
            })?;

        let mut stream = TcpStream::connect_timeout(&address, self.connect_timeout)?;
        stream.set_read_timeout(Some(self.io_timeout))?;
        stream.set_write_timeout(Some(self.io_timeout))?;
        stream.set_nodelay(true)?;

        let mut head = format!(
            "{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nAccept: application/json\r\nContent-Type: application/json\r\nContent-Length: {}\r\n",
            request.method.as_str(),
            request.path,
            host_header(&endpoint.host, endpoint.port),
            request.body.len()
        );
        if let Some(token) = &request.token {
            head.push_str("X-Till-Token: ");
            head.push_str(token);
            head.push_str("\r\n");
        }
        head.push_str("\r\n");

        stream.write_all(head.as_bytes())?;
        stream.write_all(&request.body)?;
        stream.flush()?;

        // `Connection: close` means EOF is the end of the message. Cap the
        // whole read so a peer cannot make the till allocate without bound.
        const HEADER_LIMIT: usize = 64 * 1024;
        let ceiling = self.max_response.saturating_add(HEADER_LIMIT);
        let mut raw = Vec::new();
        let mut chunk = [0u8; 8192];
        loop {
            let read = stream.read(&mut chunk)?;
            if read == 0 {
                break;
            }
            raw.extend_from_slice(&chunk[..read]);
            if raw.len() > ceiling {
                return Err(Error::TooLarge);
            }
        }

        parse_response(&raw, self.max_response)
    }
}

fn host_header(host: &str, port: u16) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

fn parse_response(raw: &[u8], max_response: usize) -> Result<Response, Error> {
    let boundary = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| Error::Malformed("response has no header terminator".to_owned()))?;
    let headers = std::str::from_utf8(&raw[..boundary])
        .map_err(|_| Error::Malformed("response headers are not UTF-8".to_owned()))?;
    let mut body = raw[boundary + 4..].to_vec();

    let mut lines = headers.split("\r\n");
    let status_line = lines.next().unwrap_or_default();
    let mut parts = status_line.split_whitespace();
    let version = parts.next().unwrap_or_default();
    if !version.starts_with("HTTP/") {
        return Err(Error::Malformed("response has no HTTP version".to_owned()));
    }
    let status: u16 = parts
        .next()
        .and_then(|code| code.parse().ok())
        .ok_or_else(|| Error::Malformed("response has no status code".to_owned()))?;

    let mut content_length = None;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        if name.eq_ignore_ascii_case("content-length") {
            content_length = Some(
                value
                    .parse::<usize>()
                    .map_err(|_| Error::Malformed("bad Content-Length".to_owned()))?,
            );
        }
        if name.eq_ignore_ascii_case("transfer-encoding") && value.eq_ignore_ascii_case("chunked") {
            return Err(Error::Malformed(
                "chunked responses are not supported".to_owned(),
            ));
        }
    }

    if let Some(length) = content_length {
        if length > max_response {
            return Err(Error::TooLarge);
        }
        if body.len() < length {
            return Err(Error::Malformed(format!(
                "response body is truncated: {} of {length} bytes",
                body.len()
            )));
        }
        body.truncate(length);
    }
    if body.len() > max_response {
        return Err(Error::TooLarge);
    }

    if !(200..300).contains(&status) {
        return Err(Error::Status { status, body });
    }
    Ok(Response { status, body })
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    use tiny_http::Server;

    use super::*;

    fn serve(answer: impl FnOnce(tiny_http::Request) + Send + 'static) -> (u16, JoinHandle<()>) {
        let server = Server::http(("127.0.0.1", 0)).expect("bind");
        let port = server.server_addr().to_ip().expect("ip").port();
        let worker = thread::spawn(move || {
            let request = server.recv().expect("one request");
            answer(request);
        });
        (port, worker)
    }

    fn endpoint(port: u16) -> Endpoint {
        Endpoint::new("127.0.0.1", port)
    }

    #[test]
    fn sends_method_path_token_and_body() {
        let (port, worker) = serve(|mut request| {
            let mut body = String::new();
            request.as_reader().read_to_string(&mut body).expect("body");
            let token = request
                .headers()
                .iter()
                .find(|header| header.field.equiv("X-Till-Token"))
                .map(|header| header.value.as_str().to_owned())
                .unwrap_or_default();
            let echoed = format!(
                "{} {} {token} {body}",
                request.method().as_str(),
                request.url()
            );
            request
                .respond(tiny_http::Response::from_string(echoed))
                .expect("reply");
        });

        let response = Client::new()
            .request(
                &endpoint(port),
                &Request::post("/v1/echo", "hello").with_token("7QK4M2X9AB"),
            )
            .expect("response");

        assert_eq!(response.status, 200);
        assert_eq!(
            String::from_utf8(response.body).unwrap(),
            "POST /v1/echo 7QK4M2X9AB hello"
        );
        worker.join().unwrap();
    }

    #[test]
    fn a_non_2xx_answer_is_an_error_that_keeps_the_reason() {
        let (port, worker) = serve(|request| {
            request
                .respond(
                    tiny_http::Response::from_string("pairing token is wrong")
                        .with_status_code(401),
                )
                .expect("reply");
        });

        let error = Client::new()
            .request(&endpoint(port), &Request::get("/v1/ping"))
            .expect_err("401 is not success");
        match error {
            Error::Status { status, body } => {
                assert_eq!(status, 401);
                assert_eq!(body, b"pairing token is wrong");
            }
            other => panic!("expected a status error, got {other:?}"),
        }
        worker.join().unwrap();
    }

    #[test]
    fn a_truncated_response_body_is_not_accepted() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
        let port = listener.local_addr().unwrap().port();
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            // Read the request first: answering while the client is still
            // writing turns the close into a reset, and the client would see a
            // transport error instead of a truncated body.
            let mut request = [0u8; 4096];
            let _ = stream.read(&mut request);
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\nshort")
                .expect("write");
        });

        let error = Client::new()
            .request(&endpoint(port), &Request::get("/"))
            .expect_err("short body");
        assert!(matches!(error, Error::Malformed(_)), "{error:?}");
        worker.join().unwrap();
    }

    #[test]
    fn a_silent_peer_times_out_instead_of_hanging() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
        let port = listener.local_addr().unwrap().port();
        let worker = thread::spawn(move || {
            let (_stream, _) = listener.accept().expect("accept");
            thread::sleep(Duration::from_millis(300));
        });

        let client = Client::with_limits(
            Duration::from_secs(1),
            Duration::from_millis(100),
            1024 * 1024,
        );
        let error = client
            .request(&endpoint(port), &Request::get("/"))
            .expect_err("timeout");
        assert!(matches!(error, Error::Io(_)), "{error:?}");
        worker.join().unwrap();
    }

    #[test]
    fn a_response_over_the_cap_is_refused() {
        let (port, worker) = serve(|request| {
            request
                .respond(tiny_http::Response::from_string("x".repeat(2048)))
                .expect("reply");
        });

        let client = Client::with_limits(Duration::from_secs(1), Duration::from_secs(1), 1024);
        let error = client
            .request(&endpoint(port), &Request::get("/"))
            .expect_err("too large");
        assert!(matches!(error, Error::TooLarge), "{error:?}");
        worker.join().unwrap();
    }

    #[test]
    fn an_invalid_path_or_token_never_opens_a_socket() {
        let endpoint = endpoint(9); // discard port: nothing listens there
        for request in [
            Request::get("v1/ping"),
            Request::get("/v1/ping\nHost: evil"),
            Request::get("/v1/ping").with_token("not-a-token"),
        ] {
            let error = Client::new()
                .request(&endpoint, &request)
                .expect_err("rejected before connecting");
            assert!(matches!(error, Error::Malformed(_)), "{error:?}");
        }
    }
}
