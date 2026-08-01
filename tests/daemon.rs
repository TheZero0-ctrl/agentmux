//! Live daemon vertical-slice regressions.

use std::error::Error;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use agentmux::daemon::api::{self, SharedDaemonState};
use agentmux::daemon::evidence::{self, EvidenceParseError};
use agentmux::daemon::state::DaemonState;
use agentmux::model::{AgentState, Pane, PaneId, ProcessEvidence};
use agentmux::state::{AgentSnapshot, normalize_snapshot};

#[test]
fn given_hook_waiting_evidence_when_reduced_then_it_overlays_fallback_state()
-> Result<(), Box<dyn Error>> {
    // Given: a fallback pane that is only idle according to tmux/process discovery.
    let mut state = DaemonState::default();
    state.reconcile_fallback(snapshot_with_pane(
        "%1",
        ProcessEvidence::live(101, "bash"),
    )?);

    // When: authoritative hook evidence says that pane is waiting for permission.
    state.ingest(&evidence::parse_evidence_line(
        "agentmux.v1 source=hook agent_id=pane:%1 state=waiting_permission sequence=42",
    )?);

    // Then: the effective row uses the authoritative waiting state and high confidence hook evidence.
    let body = api::render_state_body(&state);
    assert!(body.contains("pane:%1\tunknown\t0\tunknown\t%1"));
    assert!(body.contains("\twaiting_permission\thook\tfresh\thigh"));
    Ok(())
}

#[test]
fn given_lower_source_and_clear_events_when_reduced_then_precedence_and_sequence_are_enforced()
-> Result<(), Box<dyn Error>> {
    // Given: structured log evidence is newer by sequence but lower precedence than hook evidence.
    let mut state = DaemonState::default();
    state.reconcile_fallback(snapshot_with_pane(
        "%1",
        ProcessEvidence::live(101, "bash"),
    )?);
    for line in [
        "agentmux.v1 source=hook agent_id=pane:%1 state=waiting_permission sequence=2",
        "agentmux.v1 source=structured_log agent_id=pane:%1 state=waiting_question sequence=99",
        "agentmux.v1 source=hook agent_id=pane:%1 state=clear sequence=3",
    ] {
        state.ingest(&evidence::parse_evidence_line(line)?);
    }

    // When: callers inspect the effective snapshot after the clear event.
    let snapshot = state.effective_snapshot();

    // Then: clear removes the hook waiting override and the lower source never reasserts stale state.
    assert_eq!(snapshot.agent_state("pane:%1"), Some(AgentState::Idle));
    Ok(())
}

#[test]
fn given_missing_fallback_pane_when_reduced_then_waiting_evidence_does_not_keep_it_alive()
-> Result<(), Box<dyn Error>> {
    // Given: an authoritative waiting event exists for a previously live pane.
    let mut state = DaemonState::default();
    let first = snapshot_with_pane("%1", ProcessEvidence::live(101, "bash"))?;
    let missing = normalize_snapshot(&first, Vec::new());
    state.reconcile_fallback(first);
    state.ingest(&evidence::parse_evidence_line(
        "agentmux.v1 source=hook agent_id=pane:%1 state=waiting_permission sequence=1",
    )?);

    // When: fallback marks the pane missing/exited.
    state.reconcile_fallback(missing);

    // Then: exited fallback state wins over authoritative waiting evidence.
    assert_eq!(
        state.effective_snapshot().agent_state("pane:%1"),
        Some(AgentState::Exited)
    );
    Ok(())
}

#[test]
fn given_invalid_evidence_when_parsed_then_errors_are_sanitized() -> Result<(), Box<dyn Error>> {
    // Given: a raw payload containing private strings in unsupported fields and values.
    let secret = "agentmux.v1 source=/home/alice/token agent_id=pane:%1 state=waiting_permission sequence=1 raw_prompt=diff --git secret";

    // When: the evidence parser rejects the line.
    let error = match evidence::parse_evidence_line(secret) {
        Ok(_event) => return Err(std::io::Error::other("evidence unexpectedly parsed").into()),
        Err(error) => error,
    };

    // Then: public errors describe the field category without retaining raw payload text.
    assert!(matches!(error, EvidenceParseError::InvalidField("source")));
    assert!(!format!("{error:?}").contains("alice"));
    assert!(!error.to_string().contains("token"));
    Ok(())
}

#[test]
fn given_loopback_api_when_state_and_evidence_are_requested_then_responses_stay_private()
-> Result<(), Box<dyn Error>> {
    // Given: a loopback API server with one fallback row.
    let addr = spawn_api(snapshot_with_pane(
        "%1",
        ProcessEvidence::live(101, "bash"),
    )?)?;

    // When: evidence is posted and state is fetched.
    let post = request(
        addr,
        "POST /evidence HTTP/1.1",
        "agentmux.v1 source=hook agent_id=pane:%1 state=waiting_permission sequence=7\n",
    )?;
    let state = request(addr, "GET /state HTTP/1.1", "")?;

    // Then: the API accepts evidence and returns only projection-safe state rows.
    assert!(post.starts_with("HTTP/1.1 202 Accepted"));
    assert!(state.contains("agentmux state"));
    assert!(state.contains("waiting_permission\thook\tfresh\thigh"));
    for forbidden in ["raw_prompt", "diff --git", "/home/alice", "token="] {
        assert!(!state.contains(forbidden));
    }
    Ok(())
}

#[test]
fn given_private_invalid_evidence_when_posted_then_api_error_does_not_echo_payload()
-> Result<(), Box<dyn Error>> {
    // Given: a loopback API server and an invalid body carrying private-looking text.
    let addr = spawn_api(snapshot_with_pane(
        "%1",
        ProcessEvidence::live(101, "bash"),
    )?)?;

    // When: invalid evidence is posted.
    let response = request(
        addr,
        "POST /evidence HTTP/1.1",
        "agentmux.v1 source=/home/alice/token agent_id=pane:%1 state=waiting_permission sequence=7 raw_prompt=diff --git secret\n",
    )?;

    // Then: the response is sanitized and never echoes raw evidence payload text.
    assert!(response.starts_with("HTTP/1.1 400 Bad Request"));
    assert!(response.contains("invalid evidence field: source"));
    for forbidden in ["/home/alice", "token", "raw_prompt", "diff --git"] {
        assert!(!response.contains(forbidden));
    }
    Ok(())
}

#[test]
fn given_loopback_api_when_events_are_requested_then_current_snapshot_streams_immediately()
-> Result<(), Box<dyn Error>> {
    // Given: a loopback API server with one fallback row.
    let addr = spawn_api(snapshot_with_pane(
        "%1",
        ProcessEvidence::live(101, "bash"),
    )?)?;

    // When: a client opens the SSE endpoint.
    let mut stream = TcpStream::connect(addr)?;
    stream.set_read_timeout(Some(Duration::from_millis(500)))?;
    stream.write_all(b"GET /events HTTP/1.1\r\nHost: agentmux\r\nConnection: close\r\n\r\n")?;
    let mut output = String::new();
    let _result = stream.read_to_string(&mut output);

    // Then: the current snapshot is sent as SSE data before any later revision changes.
    assert!(output.contains("Content-Type: text/event-stream"));
    assert!(output.contains("event: snapshot"));
    assert!(output.contains("data: agentmux state"));
    Ok(())
}

#[test]
fn given_non_loopback_bind_when_validated_then_it_is_rejected() -> Result<(), Box<dyn Error>> {
    // Given: a non-loopback socket address.
    let addr = "0.0.0.0:47631".parse::<SocketAddr>()?;

    // When: daemon bind validation runs.
    let result = api::validate_loopback_addr(addr);

    // Then: the daemon refuses to expose a non-loopback listener.
    assert!(matches!(result, Err(api::ApiError::NonLoopbackBind)));
    Ok(())
}

fn snapshot_with_pane(
    pane_id: &str,
    process: ProcessEvidence,
) -> Result<AgentSnapshot, Box<dyn Error>> {
    Ok(normalize_snapshot(
        &AgentSnapshot::default(),
        [Pane::new(PaneId::new(pane_id)?, process)],
    ))
}

fn spawn_api(snapshot: AgentSnapshot) -> Result<SocketAddr, Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    let mut daemon_state = DaemonState::default();
    daemon_state.reconcile_fallback(snapshot);
    let shared = Arc::new(SharedDaemonState::new(daemon_state));
    thread::spawn(move || {
        let _result = api::serve(&listener, &shared);
    });
    Ok(addr)
}

fn request(addr: SocketAddr, first_line: &str, body: &str) -> Result<String, Box<dyn Error>> {
    let mut stream = TcpStream::connect(addr)?;
    let request = format!(
        "{first_line}\r\nHost: agentmux\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes())?;
    let mut output = String::new();
    stream.read_to_string(&mut output)?;
    Ok(output)
}
