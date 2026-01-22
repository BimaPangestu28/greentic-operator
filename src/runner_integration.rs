use std::path::Path;
use std::process::Command;

use serde_json::Value;

pub struct RunnerOutput {
    pub status: std::process::ExitStatus,
    pub stdout: String,
    pub stderr: String,
    pub parsed: Option<Value>,
}

pub fn run_flow(
    runner: &Path,
    pack: &Path,
    flow: &str,
    input: &Value,
) -> anyhow::Result<RunnerOutput> {
    let input_str = serde_json::to_string(input)?;
    let output = Command::new(runner)
        .args(["run", "--pack"])
        .arg(pack)
        .args(["--flow", flow, "--input"])
        .arg(&input_str)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let parsed = serde_json::from_str(&stdout).ok();

    Ok(RunnerOutput {
        status: output.status,
        stdout,
        stderr,
        parsed,
    })
}
