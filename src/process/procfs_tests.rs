#[cfg(unix)]
use std::ffi::OsString;
#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;

use crate::model::ProcessIdentity;

use super::procfs::{ProcessTreeSource, ProcfsReadError, parse_stat, parse_stat_bytes};
use super::procfs_test_support::{
    FakeProcfsAccess, FixtureProcess, basenames, fixture, identities, stat_line,
};

#[test]
fn given_shell_agent_child_fixture_when_read_then_tree_records_are_rooted_and_ordered() {
    // Given: a pane shell with an agent child and grandchild plus unrelated processes.
    let access = FakeProcfsAccess::new([
        fixture(100, 1, 10, "bash", Ok("bash")),
        fixture(200, 100, 20, "opencode", Ok("opencode")),
        fixture(150, 100, 15, "helper", Ok("helper")),
        fixture(300, 200, 30, "child", Ok("child")),
        fixture(900, 1, 90, "other", Ok("other")),
    ]);

    // When: the process tree is read from the pane PID.
    let records = ProcessTreeSource::new(access).read_tree(100);

    // Then: only root descendants are returned in deterministic PID order.
    assert_eq!(
        identities(&records),
        [(100, 10), (150, 15), (200, 20), (300, 30)]
    );
    assert_eq!(
        basenames(&records),
        [
            Some("bash"),
            Some("helper"),
            Some("opencode"),
            Some("child")
        ]
    );
}

#[test]
fn given_duplicate_fixture_pid_when_read_then_pid_listing_matches_constructed_map() {
    // Given: duplicate fixtures for the same PID where the retained map entry has sequential stats.
    let access = FakeProcfsAccess::new([
        fixture(10, 1, 10, "old", Ok("old")),
        FixtureProcess::with_stat_sequence(
            10,
            [
                stat_line(10, "retained", 1, 20),
                stat_line(10, "reused", 1, 30),
            ],
            Ok("retained"),
        ),
    ]);

    // When: the process tree is read from the duplicated PID.
    let records = ProcessTreeSource::new(access).read_tree(10);

    // Then: the retained fixture is read once from the internally consistent PID set.
    assert_eq!(identities(&records), [(10, 20)]);
}

#[test]
fn given_deep_descendant_chain_when_read_then_traversal_remains_ordered() {
    // Given: a descendant chain deep enough to prove traversal without sibling reordering.
    let access = FakeProcfsAccess::new([
        fixture(10, 1, 10, "p10", Ok("p10")),
        fixture(11, 10, 11, "p11", Ok("p11")),
        fixture(12, 11, 12, "p12", Ok("p12")),
        fixture(13, 12, 13, "p13", Ok("p13")),
        fixture(14, 13, 14, "p14", Ok("p14")),
        fixture(15, 14, 15, "p15", Ok("p15")),
        fixture(16, 15, 16, "p16", Ok("p16")),
        fixture(17, 16, 17, "p17", Ok("p17")),
        fixture(18, 17, 18, "p18", Ok("p18")),
        fixture(19, 18, 19, "p19", Ok("p19")),
        fixture(20, 19, 20, "p20", Ok("p20")),
        fixture(21, 20, 21, "p21", Ok("p21")),
        fixture(22, 21, 22, "p22", Ok("p22")),
        fixture(23, 22, 23, "p23", Ok("p23")),
        fixture(24, 23, 24, "p24", Ok("p24")),
        fixture(25, 24, 25, "p25", Ok("p25")),
        fixture(26, 25, 26, "p26", Ok("p26")),
        fixture(27, 26, 27, "p27", Ok("p27")),
        fixture(28, 27, 28, "p28", Ok("p28")),
        fixture(29, 28, 29, "p29", Ok("p29")),
        fixture(30, 29, 30, "p30", Ok("p30")),
    ]);

    // When: the process tree is read from the chain root.
    let records = ProcessTreeSource::new(access).read_tree(10);

    // Then: every reachable descendant is returned once in deterministic preorder.
    assert_eq!(
        identities(&records),
        [
            (10, 10),
            (11, 11),
            (12, 12),
            (13, 13),
            (14, 14),
            (15, 15),
            (16, 16),
            (17, 17),
            (18, 18),
            (19, 19),
            (20, 20),
            (21, 21),
            (22, 22),
            (23, 23),
            (24, 24),
            (25, 25),
            (26, 26),
            (27, 27),
            (28, 28),
            (29, 29),
            (30, 30),
        ]
    );
}

#[test]
fn given_root_only_fixture_when_read_then_root_record_is_returned() {
    // Given: a pane process without descendants.
    let access = FakeProcfsAccess::new([fixture(42, 1, 700, "zsh", Ok("zsh"))]);

    // When: the process tree is read from the pane PID.
    let records = ProcessTreeSource::new(access).read_tree(42);

    // Then: the root record is retained with its PID/start-time identity.
    assert_eq!(identities(&records), [(42, 700)]);
    assert_eq!(basenames(&records), [Some("zsh")]);
}

#[test]
fn given_stat_comm_with_spaces_and_parentheses_when_parsed_then_field_22_start_time_is_read() {
    // Given: procfs stat where comm contains spaces and parentheses.
    let stat = stat_line(321, "agent (worker) main", 100, 88_123);

    // When: the stat text is parsed.
    let parsed = parse_stat(321, &stat).expect("stat parses");

    // Then: PID, PPID, comm, and field-22 start time are extracted correctly.
    assert_eq!(parsed.pid(), 321);
    assert_eq!(parsed.ppid(), 100);
    assert_eq!(parsed.comm(), "agent (worker) main");
    assert_eq!(parsed.identity(), ProcessIdentity::new(321, 88_123));
}

#[test]
fn given_non_utf8_stat_bytes_when_parsed_then_ascii_identity_fields_are_preserved() {
    // Given: procfs stat bytes with a non-UTF8 byte inside the comm field.
    let fields = [
        "S", "100", "0", "0", "0", "0", "0", "0", "0", "0", "0", "0", "0", "0", "0", "0", "0", "0",
        "0", "88123",
    ];
    let mut stat = b"321 (agent ".to_vec();
    stat.push(0xff);
    stat.extend_from_slice(format!(") {}", fields.join(" ")).as_bytes());

    // When: the bytes are converted lossily at the procfs boundary and parsed.
    let parsed = parse_stat_bytes(321, &stat).expect("lossy stat parses");

    // Then: ASCII PID, PPID, and start-time fields remain available.
    assert_eq!(parsed.pid(), 321);
    assert_eq!(parsed.ppid(), 100);
    assert_eq!(parsed.identity(), ProcessIdentity::new(321, 88_123));
}

#[test]
fn given_malformed_or_truncated_stat_when_read_then_bad_records_degrade_without_panic() {
    // Given: one valid root, one malformed child, and one truncated child.
    let access = FakeProcfsAccess::new([
        fixture(10, 1, 10, "root", Ok("root")),
        FixtureProcess::with_stat(20, String::from("20 (broken) S 10"), Ok("broken")),
        FixtureProcess::with_stat(30, String::from("30 broken) S 10 0 0 0"), Ok("broken")),
    ]);

    // When: the process tree is read.
    let records = ProcessTreeSource::new(access).read_tree(10);

    // Then: malformed records are skipped and the valid root remains.
    assert_eq!(identities(&records), [(10, 10)]);
}

#[test]
fn given_missing_root_stat_with_readable_child_when_read_then_child_remains_reachable() {
    // Given: the pane root stat disappears but its child is still readable in the procfs snapshot.
    let access = FakeProcfsAccess::new([
        FixtureProcess::with_stat_error(10, ProcfsReadError::NotFound),
        fixture(20, 10, 20, "opencode", Ok("opencode")),
    ]);

    // When: the process tree is read from the missing root.
    let records = ProcessTreeSource::new(access).read_tree(10);

    // Then: the missing root is skipped but readable descendants are still returned.
    assert_eq!(identities(&records), [(20, 20)]);
    assert_eq!(basenames(&records), [Some("opencode")]);
}

#[test]
fn given_disappearing_and_permission_denied_processes_when_read_then_available_records_remain() {
    // Given: PID listing includes vanished children and inaccessible executable metadata.
    let access = FakeProcfsAccess::new([
        fixture(10, 1, 10, "root", Ok("root")),
        FixtureProcess::with_stat_error(20, ProcfsReadError::NotFound),
        fixture(
            30,
            10,
            30,
            "private",
            Err(ProcfsReadError::PermissionDenied),
        ),
        fixture(40, 10, 40, "gone-exe", Err(ProcfsReadError::NotFound)),
    ]);

    // When: the process tree is read.
    let records = ProcessTreeSource::new(access).read_tree(10);

    // Then: disappeared stats are omitted and unavailable basenames degrade to None.
    assert_eq!(identities(&records), [(10, 10), (30, 30), (40, 40)]);
    assert_eq!(basenames(&records), [Some("root"), None, None]);
}

#[test]
fn given_pid_reused_after_exe_read_when_read_then_basename_is_discarded() {
    // Given: stat identity changes after the executable basename was read for the same PID.
    let access = FakeProcfsAccess::new([FixtureProcess::with_stat_sequence(
        10,
        [stat_line(10, "root", 1, 10), stat_line(10, "reused", 1, 20)],
        Ok("opencode"),
    )]);

    // When: procfs records the tree.
    let records = ProcessTreeSource::new(access).read_tree(10);

    // Then: the original identity remains but basename evidence from the reused PID is discarded.
    assert_eq!(identities(&records), [(10, 10)]);
    assert_eq!(basenames(&records), [None]);
}

#[test]
fn given_missing_ancestor_and_cycle_when_read_then_only_reachable_visited_records_are_returned() {
    // Given: a reachable cycle plus a process whose missing parent is outside the root tree.
    let access = FakeProcfsAccess::new([
        fixture(10, 20, 10, "root", Ok("root")),
        fixture(20, 10, 20, "child", Ok("child")),
        fixture(30, 999, 30, "orphan", Ok("orphan")),
    ]);

    // When: the process tree is read from the pane PID.
    let records = ProcessTreeSource::new(access).read_tree(10);

    // Then: cycle protection returns each reachable process once and excludes the orphan.
    assert_eq!(identities(&records), [(10, 10), (20, 20)]);
}

#[cfg(unix)]
#[test]
fn given_non_utf8_exe_basename_when_read_then_basename_degrades_to_none() {
    // Given: a live process whose executable basename is not valid UTF-8.
    let fixture = FixtureProcess::with_os_basename(
        7,
        stat_line(7, "root", 1, 70),
        OsString::from_vec(vec![0xff]),
    );
    let access = FakeProcfsAccess::new([fixture]);

    // When: the process tree is read.
    let records = ProcessTreeSource::new(access).read_tree(7);

    // Then: identity remains available and the executable basename is absent.
    assert_eq!(identities(&records), [(7, 70)]);
    assert_eq!(basenames(&records), [None]);
}

#[test]
fn given_same_pid_different_start_time_when_read_then_identity_uses_start_time() {
    // Given: two separate procfs snapshots with the same PID and different start times.
    let first = ProcessTreeSource::new(FakeProcfsAccess::new([fixture(
        77,
        1,
        1000,
        "bash",
        Ok("bash"),
    )]));
    let second = ProcessTreeSource::new(FakeProcfsAccess::new([fixture(
        77,
        1,
        2000,
        "bash",
        Ok("bash"),
    )]));

    // When: each tree is read.
    let first_record = first
        .read_tree(77)
        .into_iter()
        .next()
        .expect("first record exists");
    let second_record = second
        .read_tree(77)
        .into_iter()
        .next()
        .expect("second record exists");

    // Then: PID reuse is distinguishable by start-time ticks.
    assert_ne!(first_record.identity(), second_record.identity());
    assert_eq!(
        first_record.identity().pid(),
        second_record.identity().pid()
    );
}
