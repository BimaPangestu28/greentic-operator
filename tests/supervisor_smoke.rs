use std::collections::BTreeMap;
use std::path::PathBuf;

use greentic_operator::runtime_state::RuntimePaths;
use greentic_operator::supervisor::{
    ServiceId, ServiceSpec, read_status, spawn_service, stop_service,
};

#[test]
fn supervisor_spawns_and_stops_service() {
    let temp = tempfile::tempdir().unwrap();
    let state_dir = temp.path().join("state");
    let paths = RuntimePaths::new(&state_dir, "demo", "default");

    let bin = PathBuf::from(env!("CARGO_BIN_EXE_fake_service"));
    let spec = ServiceSpec {
        id: ServiceId::new("fake").unwrap(),
        argv: vec![bin.display().to_string(), "2".to_string()],
        cwd: None,
        env: BTreeMap::new(),
    };

    let handle = spawn_service(&paths, spec, None).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let mut log_contents = String::new();
    while std::time::Instant::now() < deadline {
        if let Ok(contents) = std::fs::read_to_string(&handle.log_path) {
            if contents.contains("ready") {
                log_contents = contents;
                break;
            }
            log_contents = contents;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(
        log_contents.contains("ready"),
        "expected ready in log, got: {log_contents}"
    );

    let statuses = read_status(&paths).unwrap();
    assert_eq!(statuses.len(), 1);
    assert!(statuses[0].running);

    stop_service(&paths, &ServiceId::new("fake").unwrap(), 500).unwrap();
    let statuses = read_status(&paths).unwrap();
    assert!(statuses.is_empty());
}
