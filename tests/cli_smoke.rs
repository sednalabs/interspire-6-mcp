use std::process::Command;

#[test]
fn audience_hygiene_export_help_exits_cleanly() {
    let output = Command::new(env!("CARGO_BIN_EXE_interspire-mcp"))
        .args(["audience-hygiene-export", "--help"])
        .output()
        .unwrap_or_else(|err| panic!("run cli help: {err}"));

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap_or_else(|err| panic!("{err}"));
    assert!(stdout.contains("audience-hygiene-export"));
    assert!(stdout.contains("--source-list-ids"));
    assert!(stdout.contains("INTERSPIRE_AUDIENCE_HYGIENE_ROOTS"));
    assert!(stdout.contains("/secure/private/interspire-audience-hygiene"));
}
