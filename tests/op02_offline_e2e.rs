#[path = "support/fixture_resolver.rs"]
mod fixture_resolver;

use std::io::Write;
use std::path::Path;

use fixture_resolver::FixtureResolver;
use greentic_operator::provider_config_envelope::{
    ensure_contract_compatible, read_provider_config_envelope, write_provider_config_envelope,
};
use serde_json::Value as JsonValue;
use tempfile::tempdir;
use zip::write::FileOptions;

#[test]
fn offline_fixture_resolver_setup_upgrade_remove_with_drift_and_backup() {
    let resolver = FixtureResolver::from_root("tests/fixtures/registry").unwrap();
    let component = resolver.component("messaging-telegram").unwrap();
    component.ensure_layout().unwrap();
    assert_eq!(component.component_id(), "messaging-telegram");
    let i18n_keys = component.read_i18n_keys().unwrap().unwrap();
    assert!(!i18n_keys.is_empty());

    let temp = tempdir().unwrap();
    let providers_root = temp
        .path()
        .join("state")
        .join("runtime")
        .join("demo")
        .join("providers");
    let pack_v1 = temp.path().join("messaging-telegram-v1.gtpack");
    write_test_pack(&pack_v1, "1.0.0").unwrap();

    let setup_apply = component
        .decode_cbor_json("apply_setup_config.cbor")
        .unwrap();
    let upgrade_apply = component
        .decode_cbor_json("apply_upgrade_config.cbor")
        .unwrap();
    let remove_apply = component
        .decode_cbor_json("apply_remove_config.cbor")
        .unwrap();

    ensure_contract_compatible(
        &providers_root,
        component.component_id(),
        "setup_default",
        &pack_v1,
        false,
    )
    .unwrap();
    write_provider_config_envelope(
        &providers_root,
        component.component_id(),
        "setup_default",
        setup_apply.get("config").unwrap(),
        &pack_v1,
        false,
    )
    .unwrap();

    ensure_contract_compatible(
        &providers_root,
        component.component_id(),
        "setup_upgrade",
        &pack_v1,
        false,
    )
    .unwrap();
    let expected_setup = read_provider_config_envelope(&providers_root, component.component_id())
        .unwrap()
        .unwrap()
        .config;
    write_provider_config_envelope(
        &providers_root,
        component.component_id(),
        "setup_upgrade",
        upgrade_apply.get("config").unwrap(),
        &pack_v1,
        true,
    )
    .unwrap();
    let backup_path = providers_root
        .join(component.component_id())
        .join("config.envelope.cbor.bak");
    assert!(backup_path.exists());
    let backup_after_upgrade = decode_backup_config(&backup_path);
    assert_eq!(backup_after_upgrade, expected_setup);

    ensure_contract_compatible(
        &providers_root,
        component.component_id(),
        "setup_remove",
        &pack_v1,
        false,
    )
    .unwrap();
    let expected_upgrade = read_provider_config_envelope(&providers_root, component.component_id())
        .unwrap()
        .unwrap()
        .config;
    write_provider_config_envelope(
        &providers_root,
        component.component_id(),
        "setup_remove",
        remove_apply.get("config").unwrap(),
        &pack_v1,
        true,
    )
    .unwrap();
    let backup_after_remove = decode_backup_config(&backup_path);
    assert_eq!(backup_after_remove, expected_upgrade);

    let envelope_path = providers_root
        .join(component.component_id())
        .join("config.envelope.cbor");
    let mut envelope: greentic_operator::provider_config_envelope::ConfigEnvelope =
        serde_cbor::from_slice(&std::fs::read(&envelope_path).unwrap()).unwrap();
    envelope.describe_hash = "forced-drift-for-test".to_string();
    std::fs::write(&envelope_path, serde_cbor::to_vec(&envelope).unwrap()).unwrap();

    let drift_err = ensure_contract_compatible(
        &providers_root,
        component.component_id(),
        "setup_upgrade",
        &pack_v1,
        false,
    )
    .unwrap_err();
    assert!(drift_err.to_string().contains("OP_CONTRACT_DRIFT"));
    ensure_contract_compatible(
        &providers_root,
        component.component_id(),
        "setup_upgrade",
        &pack_v1,
        true,
    )
    .unwrap();
}

fn decode_backup_config(path: &Path) -> JsonValue {
    let bytes = std::fs::read(path).unwrap();
    let envelope: greentic_operator::provider_config_envelope::ConfigEnvelope =
        serde_cbor::from_slice(&bytes).unwrap();
    envelope.config
}

fn write_test_pack(path: &Path, version: &str) -> anyhow::Result<()> {
    let file = std::fs::File::create(path)?;
    let mut zip = zip::ZipWriter::new(file);
    zip.start_file("manifest.cbor", FileOptions::<()>::default())?;
    let manifest = serde_json::json!({
        "schema_version": "1.0.0",
        "pack_id": "messaging-telegram",
        "name": "messaging-telegram",
        "version": version,
        "kind": "provider",
        "publisher": "tests",
        "components": [{
            "id": "messaging-telegram",
            "version": version,
            "supports": ["provider"],
            "world": "greentic:component/component-v0-v6-v0@0.6.0",
            "profiles": {},
            "capabilities": { "provides": ["messaging"], "requires": [] },
            "configurators": null,
            "operations": [],
            "config_schema": {"type":"object"},
            "resources": {},
            "dev_flows": {}
        }],
        "flows": [],
        "dependencies": [],
        "capabilities": [],
        "secret_requirements": [],
        "signatures": [],
        "extensions": {}
    });
    let bytes = greentic_types::cbor::canonical::to_canonical_cbor(&manifest)
        .map_err(|err| anyhow::anyhow!("{err}"))?;
    zip.write_all(&bytes)?;
    zip.finish()?;
    Ok(())
}
