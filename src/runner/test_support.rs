use std::collections::VecDeque;
use std::io;
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, DashboardRow};

#[derive(Default)]
pub(super) struct FakeRenderer {
    pub(super) draw_count: usize,
    pub(super) drawn_rows: Vec<Vec<String>>,
}

impl super::DashboardRenderer for FakeRenderer {
    fn draw(&mut self, app: &App) -> io::Result<()> {
        self.draw_count = self.draw_count.saturating_add(1);
        self.drawn_rows.push(
            app.rows()
                .iter()
                .map(|row| row.agent_id().to_owned())
                .collect(),
        );
        Ok(())
    }
}

pub(super) struct FakeClock {
    now: std::cell::Cell<Instant>,
}

impl FakeClock {
    pub(super) fn new() -> Self {
        Self {
            now: std::cell::Cell::new(Instant::now()),
        }
    }
}

impl super::Clock for FakeClock {
    fn now(&self) -> Instant {
        self.now.get()
    }

    fn advance(&self, duration: Duration) {
        let now = self.now.get();
        let next = now.checked_add(duration).unwrap_or(now);
        self.now.set(next);
    }
}

pub(super) struct FakeDiscovery {
    pub(super) calls: usize,
    results: VecDeque<io::Result<Vec<DashboardRow>>>,
}

impl FakeDiscovery {
    pub(super) fn new(results: impl IntoIterator<Item = io::Result<Vec<DashboardRow>>>) -> Self {
        Self {
            calls: 0,
            results: results.into_iter().collect(),
        }
    }
}

impl super::DashboardDiscovery for FakeDiscovery {
    fn refresh(&mut self) -> io::Result<Vec<DashboardRow>> {
        self.calls = self.calls.saturating_add(1);
        self.results.pop_front().unwrap_or_else(|| {
            Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "fake discovery exhausted",
            ))
        })
    }
}

pub(super) struct FakeEvents {
    polls: VecDeque<FakePoll>,
    pending_event: Option<Event>,
    pub(super) timeouts: Vec<Duration>,
}

impl FakeEvents {
    pub(super) fn new(polls: impl IntoIterator<Item = FakePoll>) -> Self {
        Self {
            polls: polls.into_iter().collect(),
            pending_event: None,
            timeouts: Vec::new(),
        }
    }
}

impl super::DashboardEvents for FakeEvents {
    fn poll(&mut self, timeout: Duration, clock: &impl super::Clock) -> io::Result<bool> {
        if self.pending_event.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "ready event already pending",
            ));
        }
        self.timeouts.push(timeout);
        let poll = self
            .polls
            .pop_front()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "fake poll exhausted"))?;
        if matches!(poll.outcome, FakePollOutcome::Ready(_)) && poll.elapsed > timeout {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "ready fake poll elapsed exceeded timeout",
            ));
        }
        if matches!(poll.outcome, FakePollOutcome::Pending) {
            clock.advance(timeout);
        } else {
            clock.advance(poll.elapsed);
        }
        match poll.outcome {
            FakePollOutcome::Pending => Ok(false),
            FakePollOutcome::Ready(event) => {
                self.pending_event = Some(event);
                Ok(true)
            }
            FakePollOutcome::Error(kind) => Err(io::Error::new(kind, "event poll failed")),
        }
    }

    fn read(&mut self) -> io::Result<Event> {
        self.pending_event
            .take()
            .ok_or_else(|| io::Error::other("missing event"))
    }
}

pub(super) struct FakePoll {
    elapsed: Duration,
    outcome: FakePollOutcome,
}

impl FakePoll {
    pub(super) const fn pending() -> Self {
        Self {
            elapsed: Duration::ZERO,
            outcome: FakePollOutcome::Pending,
        }
    }

    pub(super) const fn ready(event: Event) -> Self {
        Self {
            elapsed: Duration::ZERO,
            outcome: FakePollOutcome::Ready(event),
        }
    }

    pub(super) const fn ready_after(elapsed: Duration, event: Event) -> Self {
        Self {
            elapsed,
            outcome: FakePollOutcome::Ready(event),
        }
    }

    pub(super) const fn error(kind: io::ErrorKind) -> Self {
        Self {
            elapsed: Duration::ZERO,
            outcome: FakePollOutcome::Error(kind),
        }
    }
}

enum FakePollOutcome {
    Pending,
    Ready(Event),
    Error(io::ErrorKind),
}

pub(super) fn key_event(character: char) -> Event {
    Event::Key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE))
}
