//! Std-only loopback HTTP and SSE API for daemon state.

use std::fmt::{self, Write as FmtWrite};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread;

use crate::projection::{self, HEADER};

use super::evidence::{EvidenceParseError, MAX_EVIDENCE_BODY_BYTES, parse_evidence_body};
use super::state::DaemonState;

/// Default local daemon bind address.
pub const DEFAULT_DAEMON_ADDR: &str = "127.0.0.1:47631";

/// Shared daemon state with a condition variable for SSE listeners.
#[derive(Debug, Default)]
pub struct SharedDaemonState {
    state: Mutex<DaemonState>,
    changed: Condvar,
}

impl SharedDaemonState {
    /// Create shared state from an existing reducer.
    #[must_use]
    pub const fn new(state: DaemonState) -> Self {
        Self {
            state: Mutex::new(state),
            changed: Condvar::new(),
        }
    }

    /// Apply an update and notify stream listeners.
    pub fn update(&self, update: impl FnOnce(&mut DaemonState)) -> Result<(), ApiError> {
        let changed = {
            let mut state = self.lock_state()?;
            let before = state.revision();
            update(&mut state);
            state.revision() != before
        };
        if changed {
            self.changed.notify_all();
        }
        Ok(())
    }

    fn snapshot_body(&self) -> Result<String, ApiError> {
        let state = self.lock_state()?;
        Ok(render_state_body(&state))
    }

    fn revision(&self) -> Result<u64, ApiError> {
        Ok(self.lock_state()?.revision())
    }

    fn lock_state(&self) -> Result<MutexGuard<'_, DaemonState>, ApiError> {
        self.state
            .lock()
            .map_err(|_error| ApiError::StateUnavailable)
    }

    fn wait_for_revision(&self, revision: u64) -> Result<u64, ApiError> {
        let state = self.lock_state()?;
        let state = self
            .changed
            .wait_while(state, |state| state.revision() == revision)
            .map_err(|_error| ApiError::StateUnavailable)?;
        Ok(state.revision())
    }
}

/// Sanitized API error.
#[derive(Debug)]
#[non_exhaustive]
pub enum ApiError {
    /// The daemon state lock was unavailable.
    StateUnavailable,
    /// The bind address was not loopback-only.
    NonLoopbackBind,
    /// The HTTP request was unsupported or malformed.
    BadRequest,
    /// The evidence body was not accepted.
    Evidence(EvidenceParseError),
    /// Network I/O failed.
    Io(io::Error),
}

impl fmt::Display for ApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StateUnavailable => write!(formatter, "daemon state unavailable"),
            Self::NonLoopbackBind => write!(formatter, "daemon bind address must be loopback"),
            Self::BadRequest => write!(formatter, "bad request"),
            Self::Evidence(error) => write!(formatter, "{error}"),
            Self::Io(_error) => write!(formatter, "daemon I/O failed"),
        }
    }
}

impl std::error::Error for ApiError {}

impl From<io::Error> for ApiError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<EvidenceParseError> for ApiError {
    fn from(error: EvidenceParseError) -> Self {
        Self::Evidence(error)
    }
}

/// Validate that a daemon address is loopback-only.
pub const fn validate_loopback_addr(addr: SocketAddr) -> Result<SocketAddr, ApiError> {
    match addr.ip() {
        IpAddr::V4(ip) if ip.is_loopback() => Ok(addr),
        IpAddr::V6(ip) if ip.is_loopback() => Ok(addr),
        IpAddr::V4(_) | IpAddr::V6(_) => Err(ApiError::NonLoopbackBind),
    }
}

/// Run the local daemon HTTP server forever.
pub fn serve(listener: &TcpListener, state: &Arc<SharedDaemonState>) -> Result<(), ApiError> {
    validate_loopback_addr(listener.local_addr()?)?;
    for stream in listener.incoming() {
        let stream = stream?;
        let state = Arc::clone(state);
        thread::spawn(move || {
            let _result = handle_stream(stream, &state);
        });
    }
    Ok(())
}

/// Render the current privacy-safe state body.
pub fn render_state_body(state: &DaemonState) -> String {
    let snapshot = state.effective_snapshot();
    let rows = projection::project_snapshot(&snapshot);
    let mut body = format!(
        "agentmux state\nrevision: {}\nagents: {}\n{HEADER}\n",
        state.revision(),
        rows.len()
    );
    for row in rows {
        let _result = writeln!(
            &mut body,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            row.agent_id(),
            row.session_name(),
            row.window_index(),
            row.window_name(),
            row.pane_id(),
            row.pid(),
            row.process_name(),
            row.client(),
            row.client_confidence(),
            row.workspace(),
            row.state(),
            row.evidence_source(),
            row.evidence_freshness(),
            row.evidence_confidence(),
        );
    }
    body
}

fn handle_stream(mut stream: TcpStream, state: &Arc<SharedDaemonState>) -> Result<(), ApiError> {
    let request = HttpRequest::read(&mut stream)?;
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/health") => {
            write_response(&mut stream, "200 OK", "text/plain", &health(state)?)?;
        }
        ("GET", "/state") => {
            write_response(&mut stream, "200 OK", "text/plain", &state.snapshot_body()?)?;
        }
        ("POST", "/evidence") => match ingest_evidence(state, &request.body) {
            Ok(()) => write_response(&mut stream, "202 Accepted", "text/plain", "accepted\n")?,
            Err(ApiError::Evidence(error)) => write_response(
                &mut stream,
                "400 Bad Request",
                "text/plain",
                &format!("{error}\n"),
            )?,
            Err(error) => return Err(error),
        },
        ("GET", "/events") => write_events(stream, state)?,
        ("GET" | "POST", _path) => {
            write_response(&mut stream, "404 Not Found", "text/plain", "not found\n")?;
        }
        (_method, _path) => write_response(
            &mut stream,
            "405 Method Not Allowed",
            "text/plain",
            "method not allowed\n",
        )?,
    }
    Ok(())
}

fn health(state: &SharedDaemonState) -> Result<String, ApiError> {
    Ok(format!(
        "agentmux daemon ok revision: {}\n",
        state.revision()?
    ))
}

fn ingest_evidence(state: &SharedDaemonState, body: &str) -> Result<(), ApiError> {
    let events = parse_evidence_body(body)?;
    state.update(|daemon_state| {
        for event in events {
            daemon_state.ingest(&event);
        }
    })
}

fn write_events(mut stream: TcpStream, state: &Arc<SharedDaemonState>) -> Result<(), ApiError> {
    stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n")?;
    let mut revision = state.revision()?;
    write_event(&mut stream, &state.snapshot_body()?)?;
    loop {
        revision = state.wait_for_revision(revision)?;
        write_event(&mut stream, &state.snapshot_body()?)?;
    }
}

fn write_event(stream: &mut TcpStream, body: &str) -> Result<(), ApiError> {
    stream.write_all(b"event: snapshot\n")?;
    for line in body.lines() {
        stream.write_all(b"data: ")?;
        stream.write_all(line.as_bytes())?;
        stream.write_all(b"\n")?;
    }
    stream.write_all(b"\n")?;
    stream.flush()?;
    Ok(())
}

fn write_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &str,
) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

struct HttpRequest {
    method: String,
    path: String,
    body: String,
}

impl HttpRequest {
    fn read(stream: &mut TcpStream) -> Result<Self, ApiError> {
        let mut reader = BufReader::new(stream);
        let mut first = String::new();
        reader.read_line(&mut first)?;
        let mut first_parts = first.split_ascii_whitespace();
        let method = first_parts.next().ok_or(ApiError::BadRequest)?.to_owned();
        let path = first_parts.next().ok_or(ApiError::BadRequest)?.to_owned();
        let mut content_length = 0usize;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line)?;
            if line == "\r\n" || line == "\n" {
                break;
            }
            if let Some(value) = line.strip_prefix("Content-Length:") {
                content_length = value
                    .trim()
                    .parse()
                    .map_err(|_error| ApiError::BadRequest)?;
            }
        }
        if content_length > MAX_EVIDENCE_BODY_BYTES {
            return Err(ApiError::Evidence(EvidenceParseError::BodyTooLarge));
        }
        let mut body = vec![0_u8; content_length];
        reader.read_exact(&mut body)?;
        let body = String::from_utf8(body).map_err(|_error| ApiError::BadRequest)?;
        Ok(Self { method, path, body })
    }
}
