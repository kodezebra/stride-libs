//! Bytes in, bytes out over a shop's local network.
//!
//! This crate owns the parts of an HTTP exchange that are identical in every
//! stride product: request framing, timeouts, body caps, a constant-time token
//! check, and the Crockford base32 code a person reads off one screen and types
//! on another. It owns nothing else.
//!
//! The test for whether something belongs here is one question: **does this
//! crate know what a sale (or an order, or a pupil) is?** If it does, it belongs
//! in the product. There is deliberately no `Sync` trait, no cursor, no
//! idempotency table and no SQL in this crate.
//!
//! The client is hand-rolled on [`std::net::TcpStream`] on purpose: `tiny_http`
//! is a server, and the transport is small enough that one reviewed
//! implementation is cheaper than a new dependency. Both ends speak HTTP/1.1
//! with `Connection: close` and an explicit `Content-Length`; there is no
//! keep-alive, chunking, TLS or redirect handling.

pub mod client;
pub mod pairing;
pub mod server;
pub mod token;

pub use client::{Client, Endpoint, Method, Request, Response};
pub use server::{Incoming, Reply, Server};

use std::fmt;

/// Everything that can go wrong before a product sees a response.
#[derive(Debug)]
pub enum Error {
    /// The socket failed: refused, reset, timed out.
    Io(std::io::Error),
    /// The listener could not bind the requested address.
    Bind(String),
    /// The peer spoke something that is not the HTTP we send.
    Malformed(String),
    /// A body or response crossed the caller's cap.
    TooLarge,
    /// The peer answered outside 2xx.
    Status { status: u16, body: Vec<u8> },
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "{error}"),
            Self::Bind(error) => write!(formatter, "could not listen: {error}"),
            Self::Malformed(error) => write!(formatter, "malformed HTTP: {error}"),
            Self::TooLarge => write!(formatter, "body is larger than the limit"),
            Self::Status { status, .. } => write!(formatter, "peer answered {status}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
