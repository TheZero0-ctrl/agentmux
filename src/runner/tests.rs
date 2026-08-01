use std::io;
use std::time::Duration;

use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, DashboardRow};

use super::discovery::FallbackDiscovery;
use super::test_support::{
    FakeClock, FakeDiscovery, FakeEvents, FakePoll, FakeRenderer, key_event,
};
use super::{
    Clock, DashboardDiscovery, DashboardEvents, advance_dashboard_event, run_dashboard_loop_with,
    run_with,
};

#[test]
fn given_injected_error_when_runner_is_invoked_then_error_is_propagated() {
    // Given: a runner seam that injects a failing step.
    // When: the runner executes the injected step.
    // Then: the injected error should surface to the caller.
    let result = run_with(|| Err(io::Error::other("boom")));
    assert!(result.is_err());
}

#[test]
fn given_quit_key_event_when_advanced_then_app_stops_running_and_loop_exits() {
    // Given: a running dashboard and a quit key event.
    let mut app = App::new();
    let event = Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));

    // When: the runner advances one event step.
    let continue_running = advance_dashboard_event(&mut app, &event);

    // Then: the app stops running and the loop exits.
    assert!(!app.is_running());
    assert!(!continue_running);
}

#[test]
fn given_resize_event_when_advanced_then_app_keeps_running() {
    // Given: a running dashboard and a non-key event.
    let mut app = App::new();
    let event = Event::Resize(80, 24);

    // When: the runner advances one event step.
    let continue_running = advance_dashboard_event(&mut app, &event);

    // Then: the app keeps running and the loop continues.
    assert!(app.is_running());
    assert!(continue_running);
}

#[test]
fn given_navigation_key_event_when_advanced_then_selection_moves() {
    // Given: a running dashboard with selectable rows.
    let mut app = App::new();
    app.replace_rows(vec![
        DashboardRow::for_test("pane:%1"),
        DashboardRow::for_test("pane:%2"),
    ]);
    let event = Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));

    // When: the runner advances one navigation event.
    let continue_running = advance_dashboard_event(&mut app, &event);

    // Then: the app remains running and selection moves.
    assert!(continue_running);
    assert_eq!(app.selected_index(), Some(1));
}

#[test]
fn given_help_key_event_when_advanced_then_help_visibility_toggles() {
    // Given: a running dashboard with hidden help.
    let mut app = App::new();
    let event = Event::Key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));

    // When: the runner advances one help event.
    let continue_running = advance_dashboard_event(&mut app, &event);

    // Then: the app remains running and help is visible.
    assert!(continue_running);
    assert!(app.is_help_visible());
}

#[test]
fn given_runner_starts_when_loop_runs_then_initial_refresh_precedes_first_draw() -> io::Result<()> {
    // Given: a runner with one quit event and one discovery row.
    let mut app = App::new();
    let mut discovery = FakeDiscovery::new([Ok(vec![DashboardRow::for_test("pane:%1")])]);
    let mut events = FakeEvents::new([FakePoll::ready(key_event('q'))]);
    let clock = FakeClock::new();
    let mut renderer = FakeRenderer::default();

    // When: the injected loop runs.
    run_dashboard_loop_with(&mut app, &mut discovery, &mut events, &clock, &mut renderer)?;

    // Then: the first draw observes the initially refreshed row.
    assert_eq!(renderer.drawn_rows, vec![vec!["pane:%1".to_owned()]]);
    assert_eq!(discovery.calls, 1);
    Ok(())
}

#[test]
fn given_time_before_deadline_when_loop_polls_then_no_auto_refresh_runs() -> io::Result<()> {
    // Given: a runner that quits before the one-second refresh deadline.
    let mut app = App::new();
    let mut discovery = FakeDiscovery::new([Ok(vec![DashboardRow::for_test("pane:%1")])]);
    let mut events = FakeEvents::new([FakePoll::ready(key_event('q'))]);
    let clock = FakeClock::new();
    let mut renderer = FakeRenderer::default();

    // When: one event poll is enough to quit.
    run_dashboard_loop_with(&mut app, &mut discovery, &mut events, &clock, &mut renderer)?;

    // Then: only the initial refresh occurred and polling stayed capped.
    assert_eq!(discovery.calls, 1);
    assert_eq!(events.timeouts, vec![Duration::from_millis(100)]);
    Ok(())
}

#[test]
fn given_clock_reaches_deadline_when_loop_runs_then_exactly_one_auto_refresh_occurs()
-> io::Result<()> {
    // Given: event polls advance fake time to the one-second deadline.
    let mut app = App::new();
    let mut discovery = FakeDiscovery::new([
        Ok(vec![DashboardRow::for_test("pane:%1")]),
        Ok(vec![DashboardRow::for_test("pane:%2")]),
    ]);
    let delayed_quit = FakePoll::ready_after(Duration::from_millis(100), key_event('q'));
    let mut events = FakeEvents::new((0..10).map(|_| FakePoll::pending()).chain([delayed_quit]));
    let clock = FakeClock::new();
    let mut renderer = FakeRenderer::default();

    // When: the loop runs past the automatic refresh deadline.
    run_dashboard_loop_with(&mut app, &mut discovery, &mut events, &clock, &mut renderer)?;

    // Then: the deadline caused exactly one extra refresh.
    assert_eq!(discovery.calls, 2);
    assert_eq!(app.rows(), &[DashboardRow::for_test("pane:%2")]);
    Ok(())
}

#[test]
fn given_manual_refresh_key_when_loop_runs_then_refresh_is_immediate_and_deadline_resets()
-> io::Result<()> {
    // Given: a manual refresh key followed by quit.
    let mut app = App::new();
    let mut discovery = FakeDiscovery::new([
        Ok(vec![DashboardRow::for_test("pane:%1")]),
        Ok(vec![DashboardRow::for_test("pane:%2")]),
    ]);
    let mut events = FakeEvents::new([
        FakePoll::ready(key_event('r')),
        FakePoll::ready(key_event('q')),
    ]);
    let clock = FakeClock::new();
    let mut renderer = FakeRenderer::default();

    // When: the loop handles the refresh key.
    run_dashboard_loop_with(&mut app, &mut discovery, &mut events, &clock, &mut renderer)?;

    // Then: refresh ran immediately and the next wait used the reset deadline.
    assert_eq!(discovery.calls, 2);
    assert_eq!(
        events.timeouts,
        vec![Duration::from_millis(100), Duration::from_millis(100)]
    );
    assert_eq!(app.rows(), &[DashboardRow::for_test("pane:%2")]);
    Ok(())
}

#[test]
fn given_refresh_failure_when_loop_runs_then_last_rows_are_retained_with_degraded_state()
-> io::Result<()> {
    // Given: a successful initial refresh followed by a failing manual refresh.
    let mut app = App::new();
    let mut discovery = FakeDiscovery::new([
        Ok(vec![DashboardRow::for_test("pane:%1")]),
        Err(io::Error::other(
            "tmux list-panes failed: /home/alice/private token=secret diff --git raw stderr",
        )),
    ]);
    let mut events = FakeEvents::new([
        FakePoll::ready(key_event('R')),
        FakePoll::ready(key_event('q')),
    ]);
    let clock = FakeClock::new();
    let mut renderer = FakeRenderer::default();

    // When: the failing refresh is handled.
    run_dashboard_loop_with(&mut app, &mut discovery, &mut events, &clock, &mut renderer)?;

    // Then: the previous rows remain and the degraded message is sanitized.
    assert_eq!(app.rows(), &[DashboardRow::for_test("pane:%1")]);
    assert_eq!(app.degraded_message(), Some("dashboard refresh degraded"));
    Ok(())
}

#[test]
fn given_daemon_rows_available_when_dashboard_refreshes_then_fallback_discovery_is_not_used() {
    // Given: daemon discovery returns a privacy-safe row and local discovery has a different row.
    let daemon_rows = vec![DashboardRow::for_test("pane:%9")];
    let local_rows = vec![DashboardRow::for_test("pane:%1")];
    let primary = FakeDiscovery::new([Ok(daemon_rows.clone())]);
    let fallback = FakeDiscovery::new([Ok(local_rows)]);
    let mut discovery = FallbackDiscovery::new(primary, fallback);

    // When: the dashboard refreshes through daemon-first discovery.
    let rows = discovery.refresh().expect("daemon rows are accepted");

    // Then: daemon rows win and local fallback is not consumed.
    assert_eq!(rows, daemon_rows);
    assert_eq!(discovery.primary.calls, 1);
    assert_eq!(discovery.fallback.calls, 0);
}

#[test]
fn given_daemon_unavailable_when_dashboard_refreshes_then_local_discovery_is_used() {
    // Given: daemon discovery fails and local discovery can still produce existing rows.
    let local_rows = vec![DashboardRow::for_test("pane:%1")];
    let primary = FakeDiscovery::new([Err(io::Error::other("daemon unavailable"))]);
    let fallback = FakeDiscovery::new([Ok(local_rows.clone())]);
    let mut discovery = FallbackDiscovery::new(primary, fallback);

    // When: the dashboard refreshes through daemon-first discovery.
    let rows = discovery
        .refresh()
        .expect("local fallback rows are accepted");

    // Then: the existing in-process discovery path preserves the dashboard surface.
    assert_eq!(rows, local_rows);
    assert_eq!(discovery.primary.calls, 1);
    assert_eq!(discovery.fallback.calls, 1);
}

#[test]
fn given_ready_poll_after_timeout_when_polled_then_fake_events_fail_fast_without_advancing_clock() {
    // Given: a ready fake poll whose scripted elapsed time exceeds the caller timeout.
    let mut events = FakeEvents::new([FakePoll::ready_after(
        Duration::from_millis(101),
        key_event('q'),
    )]);
    let clock = FakeClock::new();
    let before = clock.now();

    // When: the poll is executed with a shorter timeout.
    let result = events.poll(Duration::from_millis(100), &clock);

    // Then: the fake reports invalid input and does not advance time past the timeout.
    assert_eq!(
        result.map_err(|error| error.kind()),
        Err(io::ErrorKind::InvalidInput)
    );
    assert_eq!(clock.now(), before);
}

#[test]
fn given_ready_event_pending_when_poll_runs_again_then_fake_events_fail_fast() {
    // Given: fake events with an unread ready event and another scripted poll.
    let mut events = FakeEvents::new([FakePoll::ready(key_event('r')), FakePoll::pending()]);
    let clock = FakeClock::new();

    // When: callers poll twice before consuming the first event.
    let first = events.poll(Duration::ZERO, &clock);
    let second = events.poll(Duration::ZERO, &clock);

    // Then: the fake fails fast instead of silently replacing the unread event.
    assert!(matches!(first, Ok(true)));
    assert_eq!(
        second.map_err(|error| error.kind()),
        Err(io::ErrorKind::AlreadyExists)
    );
}

#[test]
fn given_event_io_error_when_loop_runs_then_error_is_returned_after_draw() {
    // Given: polling fails after the first refreshed draw.
    let mut app = App::new();
    let mut discovery = FakeDiscovery::new([Ok(vec![DashboardRow::for_test("pane:%1")])]);
    let mut events = FakeEvents::new([FakePoll::error(io::ErrorKind::Other)]);
    let clock = FakeClock::new();
    let mut renderer = FakeRenderer::default();

    // When: the injected loop runs.
    let result =
        run_dashboard_loop_with(&mut app, &mut discovery, &mut events, &clock, &mut renderer);

    // Then: the I/O error surfaces after the terminal draw boundary ran.
    assert!(result.is_err());
    assert_eq!(renderer.draw_count, 1);
}

#[test]
fn given_scripted_polls_are_exhausted_when_loop_runs_then_unexpected_eof_is_returned() {
    // Given: a runner whose fake event script omits the eventual quit or error poll.
    let mut app = App::new();
    let mut discovery = FakeDiscovery::new([Ok(vec![DashboardRow::for_test("pane:%1")])]);
    let mut events = FakeEvents::new([]);
    let clock = FakeClock::new();
    let mut renderer = FakeRenderer::default();

    // When: the injected loop asks for the first missing poll.
    let result =
        run_dashboard_loop_with(&mut app, &mut discovery, &mut events, &clock, &mut renderer);

    // Then: fake-script exhaustion fails fast instead of advancing time forever.
    assert_eq!(
        result.map_err(|error| error.kind()),
        Err(io::ErrorKind::UnexpectedEof)
    );
    assert_eq!(renderer.draw_count, 1);
}

#[test]
fn given_scripted_discovery_results_are_exhausted_when_refreshed_then_unexpected_eof_is_returned() {
    // Given: a fake discovery script with one result.
    let mut discovery = FakeDiscovery::new([Ok(vec![DashboardRow::for_test("pane:%1")])]);

    // When: callers consume the scripted result and refresh once more.
    let first = discovery.refresh();
    let exhausted = discovery.refresh();

    // Then: fake-script exhaustion fails fast instead of becoming an empty dashboard.
    assert!(first.is_ok());
    assert_eq!(
        exhausted.map_err(|error| error.kind()),
        Err(io::ErrorKind::UnexpectedEof)
    );
    assert_eq!(discovery.calls, 2);
}
