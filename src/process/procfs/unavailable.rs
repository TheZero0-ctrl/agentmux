use std::ffi::OsString;

use super::{ProcfsAccess, ProcfsReadError};

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct LinuxProcfsAccess;

impl ProcfsAccess for LinuxProcfsAccess {
    fn list_pids(&self) -> Result<Vec<u32>, ProcfsReadError> {
        Err(ProcfsReadError::Unavailable)
    }

    fn read_stat(&self, _pid: u32) -> Result<String, ProcfsReadError> {
        Err(ProcfsReadError::Unavailable)
    }

    fn read_exe_basename(&self, _pid: u32) -> Result<OsString, ProcfsReadError> {
        Err(ProcfsReadError::Unavailable)
    }
}
