use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

fn write_pack(path: &std::path::Path, pack_id: &str) -> anyhow::Result<()> {
    let file = std::fs::File::create(path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::FileOptions::<()>::default();
    zip.start_file("manifest.cbor", options)?;
    let manifest = serde_json::json!({
        "meta": {
            "pack_id": pack_id,
            "entry_flows": ["setup_default"],
        }
    });
    let bytes = serde_cbor::to_vec(&manifest)?;
    std::io::Write::write_all(&mut zip, &bytes)?;
    zip.finish()?;
    Ok(())
}

#[test]
fn demo_up_starts_events_services_when_events_packs_exist() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("providers").join("events")).unwrap();
    write_pack(
        &root.join("providers").join("events").join("events.gtpack"),
        "events-pack",
    )
    .unwrap();

    let config = format!(
        r#"services:
  events:
    enabled: auto
    components:
      - id: events-ingress
        binary: "{ingress}"
      - id: events-worker
        binary: "{worker}"
"#,
        ingress = fake_bin("fake_events_ingress").display(),
        worker = fake_bin("fake_events_worker").display(),
    );
    std::fs::write(root.join("greentic.yaml"), config).unwrap();

    let log_path = root.join("demo_start.log");
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .unwrap();
    let log_file_err = log_file.try_clone().unwrap();
    let mut child = Command::new(fake_bin("greentic-operator"))
        .args([
            "demo",
            "start",
            "--bundle",
            root.to_string_lossy().as_ref(),
            "--tenant",
            "demo",
            "--no-nats",
            "--cloudflared",
            "off",
        ])
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(log_file_err))
        .spawn()
        .unwrap();

    let mut state_ready = false;
    for _ in 0..50 {
        if root.join("state").exists() {
            state_ready = true;
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    let _ = child.kill();
    let mut exited = false;
    for _ in 0..20 {
        if let Ok(Some(_)) = child.try_wait() {
            exited = true;
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    if !exited {
        let _ = child.kill();
    }
    let _ = child.wait();
    let logs = std::fs::read_to_string(&log_path).unwrap_or_default();
    assert!(state_ready, "state dir missing. logs:\n{logs}");
}

fn fake_bin(name: &str) -> PathBuf {
    if name == "greentic-operator" {
        return PathBuf::from(env!("CARGO_BIN_EXE_greentic-operator"));
    }
    example_bin(name)
}

fn binary_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn example_bin(name: &str) -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    if path.file_name().and_then(|name| name.to_str()) == Some("deps") {
        path.pop();
    }
    let candidate = path.join("examples").join(binary_name(name));
    if candidate.exists() {
        return candidate;
    }
    let status = Command::new("cargo")
        .args(["build", "--example", name])
        .status()
        .expect("failed to build example binary");
    assert!(status.success(), "failed to build example binary");
    candidate
}
