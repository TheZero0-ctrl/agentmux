use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::ffi::OsString;

use crate::model::ProcessBasename;

use super::procfs::{ProcessRecord, ProcfsAccess, ProcfsReadError};

pub(super) fn fixture(
    process_id: u32,
    parent_process_id: u32,
    start_time: u64,
    comm: &str,
    basename: Result<&str, ProcfsReadError>,
) -> FixtureProcess {
    FixtureProcess::with_stat(
        process_id,
        stat_line(process_id, comm, parent_process_id, start_time),
        basename,
    )
}

pub(super) fn stat_line(
    process_id: u32,
    comm: &str,
    parent_process_id: u32,
    start_time: u64,
) -> String {
    let parent_process_id = parent_process_id.to_string();
    let start_time = start_time.to_string();
    let fields = [
        "S",
        &parent_process_id,
        "0",
        "0",
        "0",
        "0",
        "0",
        "0",
        "0",
        "0",
        "0",
        "0",
        "0",
        "0",
        "0",
        "0",
        "0",
        "0",
        "0",
        &start_time,
    ];
    format!("{process_id} ({comm}) {}", fields.join(" "))
}

pub(super) fn identities(records: &[ProcessRecord]) -> Vec<(u32, u64)> {
    records
        .iter()
        .map(|record| {
            (
                record.identity().pid(),
                record.identity().start_time_ticks(),
            )
        })
        .collect()
}

pub(super) fn basenames(records: &[ProcessRecord]) -> Vec<Option<&str>> {
    records
        .iter()
        .map(|record| record.basename().map(ProcessBasename::as_str))
        .collect()
}

pub(super) struct FixtureProcess {
    pid: u32,
    stats: RefCell<VecDeque<Result<String, ProcfsReadError>>>,
    fallback_stat: Result<String, ProcfsReadError>,
    basename: Result<OsString, ProcfsReadError>,
}

impl FixtureProcess {
    pub(super) fn with_stat(
        pid: u32,
        stat: String,
        basename: Result<&str, ProcfsReadError>,
    ) -> Self {
        Self::with_stat_sequence(pid, [stat], basename)
    }

    pub(super) fn with_stat_sequence<const N: usize>(
        pid: u32,
        stats: [String; N],
        basename: Result<&str, ProcfsReadError>,
    ) -> Self {
        assert!(N > 0, "stat sequence fixtures must not be empty");
        let fallback_stat = stats.last().cloned().ok_or(ProcfsReadError::NotFound);
        Self {
            pid,
            stats: RefCell::new(stats.into_iter().map(Ok).collect()),
            fallback_stat,
            basename: basename.map(OsString::from),
        }
    }

    pub(super) fn with_stat_error(pid: u32, error: ProcfsReadError) -> Self {
        Self {
            pid,
            stats: RefCell::new(VecDeque::from([Err(error.clone())])),
            fallback_stat: Err(error),
            basename: Err(ProcfsReadError::NotFound),
        }
    }

    pub(super) fn with_os_basename(pid: u32, stat: String, basename: OsString) -> Self {
        Self {
            pid,
            stats: RefCell::new(VecDeque::from([Ok(stat.clone())])),
            fallback_stat: Ok(stat),
            basename: Ok(basename),
        }
    }
}

pub(super) struct FakeProcfsAccess {
    pids: Vec<u32>,
    processes: BTreeMap<u32, FixtureProcess>,
}

impl FakeProcfsAccess {
    pub(super) fn new<const N: usize>(fixtures: [FixtureProcess; N]) -> Self {
        let processes: BTreeMap<u32, FixtureProcess> = fixtures
            .into_iter()
            .map(|fixture| (fixture.pid, fixture))
            .collect();
        let pids = processes.keys().copied().collect();
        Self { pids, processes }
    }
}

impl ProcfsAccess for FakeProcfsAccess {
    fn list_pids(&self) -> Result<Vec<u32>, ProcfsReadError> {
        Ok(self.pids.clone())
    }

    fn read_stat(&self, pid: u32) -> Result<String, ProcfsReadError> {
        self.processes
            .get(&pid)
            .map_or(Err(ProcfsReadError::NotFound), |fixture| {
                fixture
                    .stats
                    .borrow_mut()
                    .pop_front()
                    .unwrap_or_else(|| fixture.fallback_stat.clone())
            })
    }

    fn read_exe_basename(&self, pid: u32) -> Result<OsString, ProcfsReadError> {
        self.processes
            .get(&pid)
            .map_or(Err(ProcfsReadError::NotFound), |fixture| {
                fixture.basename.clone()
            })
    }
}
