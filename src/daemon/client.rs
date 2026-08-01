//! Std-only local daemon client used by dashboard fallback.

use std::fmt;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use crate::app::DashboardRow;
use crate::projection::HEADER;

/// Dashboard client for the loopback daemon API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DaemonClient {
    addr: SocketAddr,
    timeout: Duration,
}

impl DaemonClient {
    /// Create a loopback daemon client.
    #[must_use]
    pub const fn new(addr: SocketAddr, timeout: Duration) -> Self {
        Self { addr, timeout }
    }

    /// Fetch the current daemon state as dashboard rows.
    pub fn fetch_rows(&self) -> Result<Vec<DashboardRow>, ClientError> {
        let mut stream = TcpStream::connect_timeout(&self.addr, self.timeout)?;
        stream.set_read_timeout(Some(self.timeout))?;
        stream.set_write_timeout(Some(self.timeout))?;
        stream.write_all(b"GET /state HTTP/1.1\r\nHost: agentmux\r\nConnection: close\r\n\r\n")?;
        read_state_response(stream)
    }
}

/// Sanitized daemon client error.
#[derive(Debug)]
#[non_exhaustive]
pub enum ClientError {
    /// Network I/O failed.
    Io(io::Error),
    /// The daemon returned an invalid response.
    InvalidResponse,
}

impl fmt::Display for ClientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(_error) => write!(formatter, "daemon unavailable"),
            Self::InvalidResponse => write!(formatter, "daemon returned invalid state"),
        }
    }
}

impl std::error::Error for ClientError {}

impl From<io::Error> for ClientError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

fn read_state_response(stream: TcpStream) -> Result<Vec<DashboardRow>, ClientError> {
    let mut reader = BufReader::new(stream);
    let mut status = String::new();
    reader.read_line(&mut status)?;
    if !status.starts_with("HTTP/1.1 200 ") {
        return Err(ClientError::InvalidResponse);
    }
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        if line == "\r\n" || line == "\n" {
            break;
        }
    }
    let mut body = String::new();
    reader.read_to_string(&mut body)?;
    parse_state_body(&body)
}

/// Parse a daemon `/state` body into dashboard rows.
pub fn parse_state_body(body: &str) -> Result<Vec<DashboardRow>, ClientError> {
    let mut lines = body.lines();
    if lines.next() != Some("agentmux state") {
        return Err(ClientError::InvalidResponse);
    }
    let revision = lines.next().ok_or(ClientError::InvalidResponse)?;
    if !revision.starts_with("revision: ") {
        return Err(ClientError::InvalidResponse);
    }
    let agents = lines.next().ok_or(ClientError::InvalidResponse)?;
    if !agents.starts_with("agents: ") {
        return Err(ClientError::InvalidResponse);
    }
    if lines.next() != Some(HEADER) {
        return Err(ClientError::InvalidResponse);
    }
    lines
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            DashboardRow::from_tsv_fields(&fields).ok_or(ClientError::InvalidResponse)
        })
        .collect()
}
