use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::PathBuf;

use super::{ProcfsAccess, ProcfsReadError};

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct LinuxProcfsAccess;

impl ProcfsAccess for LinuxProcfsAccess {
    fn list_pids(&self) -> Result<Vec<u32>, ProcfsReadError> {
        let mut pids = Vec::new();
        for entry in fs::read_dir("/proc").map_err(|error| map_io_error(&error))? {
            let Ok(entry) = entry else {
                continue;
            };
            if let Some(pid) = entry
                .file_name()
                .to_str()
                .and_then(|name| name.parse::<u32>().ok())
            {
                pids.push(pid);
            }
        }
        Ok(pids)
    }

    fn read_stat(&self, pid: u32) -> Result<String, ProcfsReadError> {
        fs::read(proc_entry(pid, "stat"))
            .map(|bytes| super::stat_bytes_to_string(&bytes))
            .map_err(|error| map_io_error(&error))
    }

    fn read_exe_basename(&self, pid: u32) -> Result<OsString, ProcfsReadError> {
        let exe = fs::read_link(proc_entry(pid, "exe")).map_err(|error| map_io_error(&error))?;
        exe.file_name()
            .map(std::ffi::OsStr::to_os_string)
            .ok_or(ProcfsReadError::NotFound)
    }
}

fn proc_entry(pid: u32, name: &str) -> PathBuf {
    PathBuf::from("/proc").join(pid.to_string()).join(name)
}

fn map_io_error(error: &io::Error) -> ProcfsReadError {
    match error.kind() {
        io::ErrorKind::NotFound => ProcfsReadError::NotFound,
        io::ErrorKind::PermissionDenied => ProcfsReadError::PermissionDenied,
        kind => ProcfsReadError::Io(kind),
    }
}
