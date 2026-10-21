use assert_cmd::Command;

fn sift(args: &[&str]) -> Command {
    let mut cmd = Command::cargo_bin("sift").unwrap();
    cmd.env("CLAUDE_CONFIG_DIR", concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/claude"))
        .env("TZ", "UTC")
        .env_remove("NO_COLOR")
        .args(args);
    cmd
}

fn stdout(args: &[&str]) -> String {
    String::from_utf8(sift(args).assert().success().get_output().stdout.clone()).unwrap()
}

#[test]
fn formats_multiline_and_skips_bad_records() {
    let out = stdout(&["review"]);
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
    let headers = |args: &[&str]| stdout(args).matches(">>>>").count();
    assert_eq!(headers(&["draft", "unrelated"]), 2);
    assert_eq!(headers(&["-a", "review", "plan"]), 1);
    assert_eq!(headers(&["-c", "review", "--not", "draft", "--not", "clock"]), 2);
    assert_eq!(headers(&["--and", "código", "review"]), 1);
}

#[test]
fn no_match_exits_1_and_missing_history_is_not_an_error() {
    sift(&["zzz-nothing"]).assert().code(1).stdout("");
    sift(&["review"])
        .env("CLAUDE_CONFIG_DIR", "/nonexistent")
        .assert()
        .code(1)
        .stderr("");
}
