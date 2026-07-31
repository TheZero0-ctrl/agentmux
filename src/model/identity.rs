/// Linux process identity that survives PID reuse by including start-time ticks.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProcessIdentity {
    pid: u32,
    start_time_ticks: u64,
}

impl ProcessIdentity {
    /// Create a process identity from procfs PID and start-time ticks.
    #[must_use]
    pub const fn new(pid: u32, start_time_ticks: u64) -> Self {
        Self {
            pid,
            start_time_ticks,
        }
    }

    /// Return the process id.
    #[must_use]
    pub const fn pid(&self) -> u32 {
        self.pid
    }

    /// Return Linux procfs field-22 start-time ticks.
    #[must_use]
    pub const fn start_time_ticks(&self) -> u64 {
        self.start_time_ticks
    }
}
