use assert_cmd::Command;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

/// Every source is pinned to fixtures so real histories never leak in.
fn sift(args: &[&str]) -> Command {
    let mut cmd = Command::cargo_bin("sift").unwrap();
    cmd.env("CLAUDE_CONFIG_DIR", format!("{FIXTURES}/claude"))
        .env("CODEX_HOME", format!("{FIXTURES}/codex"))
        .env("PI_CODING_AGENT_DIR", format!("{FIXTURES}/pi"))
        .env_remove("PI_CODING_AGENT_SESSION_DIR")
        .env("TZ", "UTC")
        .env_remove("NO_COLOR")
        .args(args);
    cmd
}

fn stdout(args: &[&str]) -> String {
    String::from_utf8(sift(args).assert().success().get_output().stdout.clone()).unwrap()
}

/// First line of each result.
fn prompts(args: &[&str]) -> Vec<String> {
    stdout(args)
        .lines()
        .filter_map(|l| l.strip_prefix(">>>> "))
        .map(str::to_string)
        .collect()
}

#[test]
fn formats_multiline_and_skips_bad_records() {
    let out = stdout(&["-c", "review"]);
    assert_eq!(
        out,
        "CLAUDE · 2026-10-19 16:02\n\
         >>>> go ahead and review the changes and draft a PR\n\n\
         CLAUDE · 2026-10-20 16:00\n\
         >>>> review the implementation\n\
         \x20    against our original plan\n\n\
         CLAUDE · unknown time\n\
         >>>> review without a clock\n\n\
         CLAUDE · 2026-10-20 16:01\n\
         >>>> Revisión del \"código\" ✓ REVIEW\n\n"
    );
}

#[test]
fn and_or_not() {
    let count = |args: &[&str]| prompts(args).len();
    assert_eq!(count(&["-c", "draft", "unrelated"]), 2);
    assert_eq!(count(&["-c", "-a", "review", "plan"]), 1);
    assert_eq!(
        count(&["-c", "review", "--not", "draft", "--not", "clock"]),
        2
    );
    assert_eq!(count(&["-c", "--and", "código", "review"]), 1);
}

#[test]
fn codex_skips_injected_imported_and_duplicate_prompts() {
    assert_eq!(
        prompts(&["-x", "review"]),
        [
            "review the codex plan",
            "review typed after import",
            "review from history only"
        ]
    );
}

#[test]
fn pi_keeps_typed_text_only() {
    let out = stdout(&["-p", "review"]);
    assert_eq!(
        out,
        "PI · 2026-10-20 12:00\n>>>> review the pi branch\n     and its tests\n\n"
    );
}

#[test]
fn source_flags_combine_and_default_to_all() {
    let labels = |args: &[&str]| {
        let mut labels: Vec<String> = stdout(args)
            .lines()
            .filter_map(|l| l.split(" · ").next().filter(|_| l.contains(" · ")))
            .map(str::to_string)
            .collect();
        labels.dedup();
        labels
    };
    assert_eq!(labels(&["review"]), ["CLAUDE", "CODEX", "PI"]);
    assert_eq!(labels(&["-c", "-p", "review"]), ["CLAUDE", "PI"]);
    assert_eq!(labels(&["-x", "review"]), ["CODEX"]);
}

#[test]
fn no_match_exits_1_and_missing_histories_are_not_errors() {
    sift(&["zzz-nothing"]).assert().code(1).stdout("");
    sift(&["review"])
        .env("CLAUDE_CONFIG_DIR", "/nonexistent")
        .env("CODEX_HOME", "/nonexistent")
        .env("PI_CODING_AGENT_DIR", "/nonexistent")
        .assert()
        .code(1)
        .stderr("");
}

#[test]
fn pi_session_dir_override_wins() {
    sift(&["-p", "review"])
        .env("PI_CODING_AGENT_SESSION_DIR", "/nonexistent")
        .assert()
        .code(1);
}

const NEWEST: [&str; 8] = [
    "review typed after import",
    "Revisión del \"código\" ✓ REVIEW",
    "review the implementation",
    "review the pi branch",
    "review the codex plan",
    "go ahead and review the changes and draft a PR",
    "review from history only",
    "review without a clock",
];

#[test]
fn sort_merges_sources_by_time_untimed_as_oldest() {
    assert_eq!(prompts(&["review", "--sort", "newest"]), NEWEST);
    let mut oldest = NEWEST;
    oldest.reverse();
    assert_eq!(prompts(&["review", "--sort", "oldest"]), oldest);
}

#[test]
fn limit_defaults_to_newest_and_respects_sort() {
    assert_eq!(prompts(&["review", "-n", "3"]), NEWEST[..3]);
    assert_eq!(
        prompts(&["review", "--limit", "2", "--sort", "oldest"]),
        ["review without a clock", "review from history only"]
    );
}

#[test]
fn since_drops_old_and_untimed_prompts() {
    let all_timed = prompts(&["review", "--since", "1000000d"]);
    assert_eq!(all_timed.len(), NEWEST.len() - 1);
    assert!(!all_timed.contains(&"review without a clock".to_string()));
    sift(&["review", "--since", "1s"]).assert().code(1);
    sift(&["review", "--since", "7y"])
        .assert()
        .code(2)
        .stderr(predicates::str::contains("unit must be"));
}

#[test]
fn json_lines() {
    let out = stdout(&["-x", "-p", "review", "--json", "--sort", "oldest"]);
    let rows: Vec<serde_json::Value> = out
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(rows.len(), 4);
    assert_eq!(
        rows[0],
        serde_json::json!({
            "source": "codex",
            "timestamp": "2026-10-19T08:53:20Z",
            "session_id": "33333333-3333-3333-3333-333333333333",
            "text": "review from history only",
        })
    );
    assert_eq!(rows[2]["source"], "pi");
    assert_eq!(rows[2]["text"], "review the pi branch\nand its tests");
    assert_eq!(
        rows[2]["session_id"],
        "44444444-4444-4444-4444-444444444444"
    );
}
