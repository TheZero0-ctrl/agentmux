use std::path::Path;

use crate::model::{
    LocationMetadata, ModelError, Pane, PaneId, ProcessEvidence, ProcessLiveness, SessionName,
    SessionWindow, WindowIndex, WindowName, WorkspaceLabel, contains_disallowed_display_control,
};

use super::{FIELD_SEPARATOR, FieldErrorReason, TmuxError};

const EXPECTED_FIELD_COUNT: usize = 8;

const SESSION_NAME: &str = "session_name";
const WINDOW_INDEX: &str = "window_index";
const WINDOW_NAME: &str = "window_name";
const PANE_ID: &str = "pane_id";
const PANE_PID: &str = "pane_pid";
const PANE_DEAD: &str = "pane_dead";
const PANE_CURRENT_PATH: &str = "pane_current_path";
const PANE_CURRENT_COMMAND: &str = "pane_current_command";

/// Parse strict delimiter-separated `tmux list-panes` output.
///
/// # Errors
/// Returns [`TmuxError`] when any row is malformed.
pub fn parse_list_panes(output: &str) -> Result<Vec<Pane>, TmuxError> {
    let mut panes = Vec::new();

    for (row_index, row) in output.lines().enumerate() {
        panes.push(parse_row(row_index, row)?);
    }

    Ok(panes)
}

fn parse_row(row_index: usize, row: &str) -> Result<Pane, TmuxError> {
    let fields: Vec<&str> = row.split(FIELD_SEPARATOR).collect();
    let [
        session_name,
        window_index,
        window_name,
        pane_id,
        pid,
        dead,
        workspace,
        command,
    ] = fields.as_slice()
    else {
        return Err(TmuxError::MalformedFieldCount {
            row_index,
            fields: fields.len(),
        });
    };
    const { assert!(EXPECTED_FIELD_COUNT == 8) };

    let session_name = parse_session_name(row_index, session_name)?;
    let window_index = parse_window_index(row_index, window_index)?;
    let window_name = parse_window_name(row_index, window_name)?;
    let pane_id = parse_pane_id(row_index, pane_id)?;
    let pid = parse_pid(row_index, pid)?;
    let liveness = parse_liveness(row_index, dead)?;
    let workspace = parse_workspace(row_index, workspace)?;
    reject_control(row_index, PANE_CURRENT_COMMAND, command)?;

    let location = LocationMetadata::new(
        pane_id,
        SessionWindow::new(session_name, window_index, window_name),
        workspace,
    );
    Ok(Pane::with_location(
        location,
        ProcessEvidence::from_tmux(pid, command, liveness),
    ))
}

fn parse_session_name(row_index: usize, value: &str) -> Result<SessionName, TmuxError> {
    reject_control(row_index, SESSION_NAME, value)?;
    SessionName::new(value).map_err(|error| model_error(row_index, SESSION_NAME, &error))
}

fn parse_window_index(row_index: usize, value: &str) -> Result<WindowIndex, TmuxError> {
    reject_control(row_index, WINDOW_INDEX, value)?;
    value
        .parse::<u32>()
        .map(WindowIndex::new)
        .map_err(|_error| {
            field_error(
                row_index,
                WINDOW_INDEX,
                FieldErrorReason::InvalidUnsignedInteger,
            )
        })
}

fn parse_window_name(row_index: usize, value: &str) -> Result<WindowName, TmuxError> {
    reject_control(row_index, WINDOW_NAME, value)?;
    WindowName::new(value).map_err(|error| model_error(row_index, WINDOW_NAME, &error))
}

fn parse_pane_id(row_index: usize, value: &str) -> Result<PaneId, TmuxError> {
    reject_control(row_index, PANE_ID, value)?;
    PaneId::new(value)
        .map_err(|_error| field_error(row_index, PANE_ID, FieldErrorReason::InvalidPaneId))
}

fn parse_pid(row_index: usize, value: &str) -> Result<u32, TmuxError> {
    reject_control(row_index, PANE_PID, value)?;
    value.parse::<u32>().map_err(|_error| {
        field_error(
            row_index,
            PANE_PID,
            FieldErrorReason::InvalidUnsignedInteger,
        )
    })
}

fn parse_liveness(row_index: usize, value: &str) -> Result<ProcessLiveness, TmuxError> {
    reject_control(row_index, PANE_DEAD, value)?;
    match value {
        "0" => Ok(ProcessLiveness::Live),
        "1" => Ok(ProcessLiveness::Dead),
        _ => Err(field_error(
            row_index,
            PANE_DEAD,
            FieldErrorReason::InvalidBoolean,
        )),
    }
}

fn parse_workspace(row_index: usize, value: &str) -> Result<WorkspaceLabel, TmuxError> {
    reject_control(row_index, PANE_CURRENT_PATH, value)?;
    WorkspaceLabel::from_path(Path::new(value))
        .map_err(|error| model_error(row_index, PANE_CURRENT_PATH, &error))
}

fn reject_control(
    row_index: usize,
    field_name: &'static str,
    value: &str,
) -> Result<(), TmuxError> {
    if contains_disallowed_display_control(value) {
        return Err(field_error(
            row_index,
            field_name,
            FieldErrorReason::ControlCharacter,
        ));
    }
    Ok(())
}

const fn model_error(row_index: usize, field_name: &'static str, error: &ModelError) -> TmuxError {
    let reason = match error {
        ModelError::ControlCharacter => FieldErrorReason::ControlCharacter,
        ModelError::EmptyId => FieldErrorReason::Empty,
        ModelError::InvalidPaneIdFormat => FieldErrorReason::InvalidPaneId,
        ModelError::PathComponent => FieldErrorReason::PathComponent,
    };
    field_error(row_index, field_name, reason)
}

const fn field_error(
    row_index: usize,
    field_name: &'static str,
    reason: FieldErrorReason,
) -> TmuxError {
    TmuxError::MalformedField {
        row_index,
        field_name,
        reason,
    }
}
