use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fmt;

use crate::model::{ProcessBasename, ProcessIdentity};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(not(target_os = "linux"))]
mod unavailable;

#[cfg(target_os = "linux")]
type PlatformProcfsAccess = linux::LinuxProcfsAccess;
#[cfg(not(target_os = "linux"))]
type PlatformProcfsAccess = unavailable::LinuxProcfsAccess;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ProcfsReadError {
    NotFound,
    PermissionDenied,
    #[cfg(not(target_os = "linux"))]
    Unavailable,
    MalformedStat,
    Io(std::io::ErrorKind),
}

impl fmt::Display for ProcfsReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => formatter.write_str("procfs process disappeared"),
            Self::PermissionDenied => formatter.write_str("procfs permission denied"),
            #[cfg(not(target_os = "linux"))]
            Self::Unavailable => formatter.write_str("procfs unavailable on this platform"),
            Self::MalformedStat => formatter.write_str("procfs stat data is malformed"),
            Self::Io(kind) => write!(formatter, "procfs I/O failed with {}", io_kind_name(*kind)),
        }
    }
}

impl std::error::Error for ProcfsReadError {}

pub(crate) trait ProcfsAccess {
    fn list_pids(&self) -> Result<Vec<u32>, ProcfsReadError>;
    fn read_stat(&self, pid: u32) -> Result<String, ProcfsReadError>;
    fn read_exe_basename(&self, pid: u32) -> Result<OsString, ProcfsReadError>;
}

/// Standard-library Linux process-tree source.
#[derive(Clone, Debug)]
pub struct LinuxProcessTreeSource {
    inner: ProcessTreeSource<PlatformProcfsAccess>,
}

impl Default for LinuxProcessTreeSource {
    fn default() -> Self {
        Self::new()
    }
}

impl LinuxProcessTreeSource {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            inner: ProcessTreeSource::new(PlatformProcfsAccess::default()),
        }
    }

    /// Read the process tree rooted at `root_pid`.
    #[must_use]
    pub fn read_tree(&self, root_pid: u32) -> Vec<ProcessRecord> {
        self.inner.read_tree(root_pid)
    }
}

/// Privacy-safe process-tree record with PID/start-time identity and executable basename.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessRecord {
    identity: ProcessIdentity,
    basename: Option<ProcessBasename>,
}

impl ProcessRecord {
    /// Create privacy-safe process-tree evidence from parsed procfs fields.
    #[must_use]
    pub const fn new(identity: ProcessIdentity, basename: Option<ProcessBasename>) -> Self {
        Self { identity, basename }
    }

    /// Return the PID plus procfs start-time identity.
    #[must_use]
    pub const fn identity(&self) -> ProcessIdentity {
        self.identity
    }

    /// Return the executable basename when procfs exposed one safely.
    #[must_use]
    pub const fn basename(&self) -> Option<&ProcessBasename> {
        self.basename.as_ref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ParsedStat {
    pid: u32,
    ppid: u32,
    comm: String,
    identity: ProcessIdentity,
}

impl ParsedStat {
    #[cfg(test)]
    pub(crate) const fn pid(&self) -> u32 {
        self.pid
    }

    #[cfg(test)]
    pub(crate) const fn ppid(&self) -> u32 {
        self.ppid
    }

    #[cfg(test)]
    pub(crate) fn comm(&self) -> &str {
        &self.comm
    }

    #[cfg(test)]
    pub(crate) const fn identity(&self) -> ProcessIdentity {
        self.identity
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ProcessTreeSource<A> {
    access: A,
}

impl<A> ProcessTreeSource<A>
where
    A: ProcfsAccess,
{
    pub(crate) const fn new(access: A) -> Self {
        Self { access }
    }

    pub(crate) fn read_tree(&self, root_pid: u32) -> Vec<ProcessRecord> {
        let Ok(mut pids) = self.access.list_pids() else {
            return Vec::new();
        };
        pids.sort_unstable();

        let stats = self.read_stats(&pids);
        let children = child_index(&stats);
        let mut visited = BTreeSet::new();
        let mut ordered = Vec::new();
        collect_descendants(root_pid, &stats, &children, &mut visited, &mut ordered);

        ordered
            .into_iter()
            .filter_map(|pid| stats.get(&pid).map(|stat| self.record_for(stat)))
            .collect()
    }

    fn read_stats(&self, pids: &[u32]) -> BTreeMap<u32, ParsedStat> {
        pids.iter()
            .filter_map(|pid| {
                self.access
                    .read_stat(*pid)
                    .ok()
                    .and_then(|stat| parse_stat(*pid, &stat).ok())
                    .map(|stat| (*pid, stat))
            })
            .collect()
    }

    fn record_for(&self, stat: &ParsedStat) -> ProcessRecord {
        let basename = self.basename_for(stat);

        ProcessRecord::new(stat.identity, basename)
    }

    fn basename_for(&self, stat: &ParsedStat) -> Option<ProcessBasename> {
        let basename = self
            .access
            .read_exe_basename(stat.pid)
            .ok()
            .and_then(|basename| basename.into_string().ok())
            .and_then(|basename| ProcessBasename::new(&basename).ok());
        let same_identity = self
            .access
            .read_stat(stat.pid)
            .ok()
            .and_then(|latest| parse_stat(stat.pid, &latest).ok())
            .is_some_and(|latest| latest.identity == stat.identity);

        if same_identity { basename } else { None }
    }
}

pub(crate) fn stat_bytes_to_string(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[cfg(test)]
pub(crate) fn parse_stat_bytes(
    expected_pid: u32,
    stat: &[u8],
) -> Result<ParsedStat, ProcfsReadError> {
    parse_stat(expected_pid, &stat_bytes_to_string(stat))
}

pub(crate) fn parse_stat(expected_pid: u32, stat: &str) -> Result<ParsedStat, ProcfsReadError> {
    let (pid_text, after_pid) = stat
        .split_once(" (")
        .ok_or(ProcfsReadError::MalformedStat)?;
    let parsed_pid = pid_text
        .parse::<u32>()
        .map_err(|_error| ProcfsReadError::MalformedStat)?;
    if parsed_pid != expected_pid {
        return Err(ProcfsReadError::MalformedStat);
    }

    let comm_end = after_pid
        .rfind(") ")
        .ok_or(ProcfsReadError::MalformedStat)?;
    let comm = &after_pid[..comm_end];
    let fields_start = comm_end
        .checked_add(2)
        .ok_or(ProcfsReadError::MalformedStat)?;
    let fields: Vec<&str> = after_pid[fields_start..].split_whitespace().collect();
    let parent_pid = parse_field(&fields, 1)?;
    let start_time_ticks = parse_field(&fields, 19)?;

    Ok(ParsedStat {
        pid: parsed_pid,
        ppid: parent_pid,
        comm: comm.to_owned(),
        identity: ProcessIdentity::new(parsed_pid, start_time_ticks),
    })
}

const fn io_kind_name(kind: std::io::ErrorKind) -> &'static str {
    match kind {
        std::io::ErrorKind::NotFound => "not found",
        std::io::ErrorKind::PermissionDenied => "permission denied",
        std::io::ErrorKind::AlreadyExists => "already exists",
        std::io::ErrorKind::InvalidInput => "invalid input",
        std::io::ErrorKind::InvalidData => "invalid data",
        std::io::ErrorKind::TimedOut => "timed out",
        std::io::ErrorKind::Unsupported => "unsupported",
        std::io::ErrorKind::UnexpectedEof => "unexpected eof",
        _ => "other",
    }
}

fn parse_field<T>(fields: &[&str], index: usize) -> Result<T, ProcfsReadError>
where
    T: std::str::FromStr,
{
    fields
        .get(index)
        .ok_or(ProcfsReadError::MalformedStat)?
        .parse::<T>()
        .map_err(|_error| ProcfsReadError::MalformedStat)
}

fn child_index(stats: &BTreeMap<u32, ParsedStat>) -> BTreeMap<u32, BTreeSet<u32>> {
    let mut children: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for stat in stats.values() {
        children.entry(stat.ppid).or_default().insert(stat.pid);
    }
    children
}

fn collect_descendants(
    pid: u32,
    stats: &BTreeMap<u32, ParsedStat>,
    children: &BTreeMap<u32, BTreeSet<u32>>,
    visited: &mut BTreeSet<u32>,
    ordered: &mut Vec<u32>,
) {
    let mut stack = vec![pid];

    while let Some(current_pid) = stack.pop() {
        if !visited.insert(current_pid) {
            continue;
        }

        if stats.contains_key(&current_pid) {
            ordered.push(current_pid);
        }
        if let Some(child_pids) = children.get(&current_pid) {
            stack.extend(child_pids.iter().rev().copied());
        }
    }
}
