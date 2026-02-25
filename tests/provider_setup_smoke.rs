use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

use greentic_operator::config::{DemoConfig, DemoProviderConfig};
use greentic_operator::providers::{ProviderSetupOptions, run_provider_setup};

#[test]
fn provider_setup_writes_outputs() {
    let temp = tempfile::tempdir().unwrap();
    let config_dir = temp.path();
    let pack_dir = config_dir.join("provider-packs");
    std::fs::create_dir_all(&pack_dir).unwrap();
    let pack_path = pack_dir.join("msgraph.gtpack");
    std::fs::write(&pack_path, "stub").unwrap();

    let config = DemoConfig {
        tenant: "demo".to_string(),
        team: "default".to_string(),
        services: Default::default(),
        providers: Some(BTreeMap::from([(
            "msgraph".to_string(),
            DemoProviderConfig {
                pack: Some(pack_path.to_string_lossy().to_string()),
                setup_flow: Some("setup_default".to_string()),
                verify_flow: Some("verify_webhooks".to_string()),
            },
        )])),
    };

    let options = ProviderSetupOptions {
        providers: Some(vec!["msgraph".to_string()]),
        verify_webhooks: true,
        force_setup: true,
        skip_setup: false,
        skip_secrets_init: true,
        allow_contract_change: false,
        backup: false,
        setup_input: None,
        runner_binary: Some(fake_bin("fake_runner")),
        continue_on_error: true,
    };

    run_provider_setup(config_dir, &config, Some("https://example.test"), options).unwrap();

    let providers_root = config_dir
        .join("state")
        .join("runtime")
        .join("demo.default")
        .join("providers");
    assert!(providers_root.join("msgraph.setup.json").exists());
    assert!(providers_root.join("msgraph.verify.json").exists());
    assert!(providers_root.join("msgraph.status.json").exists());
    assert!(
        providers_root
            .join("msgraph")
            .join("config.envelope.cbor")
            .exists()
    );
    assert!(
        providers_root
            .join("msgraph")
            .join("answers")
            .join("setup.answers.json")
            .exists()
    );
    assert!(
        providers_root
            .join("msgraph")
            .join("answers")
            .join("setup.answers.cbor")
            .exists()
    );
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
