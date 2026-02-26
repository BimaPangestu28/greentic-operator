use std::process::Command;

fn operator_bin() -> &'static str {
    env!("CARGO_BIN_EXE_greentic-operator")
}

#[test]
fn missing_subcommand_error_is_localized_with_cli_locale() {
    let output = Command::new(operator_bin())
        .arg("--locale")
        .arg("de-DE")
        .output()
        .expect("run greentic-operator");

    assert!(
        !output.status.success(),
        "command should fail without subcommand"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("erfordert ein Unterkommando"),
        "expected german localized subcommand error, got: {stderr}"
    );
    assert!(
        stderr.contains("Verwendung:"),
        "expected localized usage label, got: {stderr}"
    );
}

#[test]
fn missing_subcommand_error_falls_back_to_english_for_unknown_locale() {
    let output = Command::new(operator_bin())
        .arg("--locale")
        .arg("zz-ZZ")
        .env("LC_ALL", "en_US.UTF-8")
        .env("LANG", "en_US.UTF-8")
        .env("LANGUAGE", "en")
        .output()
        .expect("run greentic-operator");

    assert!(
        !output.status.success(),
        "command should fail without subcommand"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("requires a subcommand"),
        "expected english fallback subcommand error, got: {stderr}"
    );
}

#[test]
fn top_level_help_is_localized_with_cli_locale() {
    let output = Command::new(operator_bin())
        .arg("--locale")
        .arg("de-DE")
        .arg("--help")
        .output()
        .expect("run greentic-operator --help");

    assert!(
        output.status.success(),
        "help should exit successfully, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Greentic-Operator-Werkzeuge"),
        "expected localized headline, got: {stdout}"
    );
    assert!(
        stdout.contains("Befehle:"),
        "expected localized commands header, got: {stdout}"
    );
    assert!(
        stdout.contains("Optionen:"),
        "expected localized options header, got: {stdout}"
    );
}
