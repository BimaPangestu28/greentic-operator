#[path = "support/fixture_resolver.rs"]
mod fixture_resolver;

use fixture_resolver::FixtureResolver;
use serde_json::Value as JsonValue;

#[test]
fn fixture_registry_matches_required_layout_and_cbor_payloads() {
    let resolver = FixtureResolver::from_root("tests/fixtures/registry").expect("load resolver");
    let component = resolver
        .component("messaging-telegram")
        .expect("resolve messaging-telegram fixture");
    assert_eq!(component.component_id(), "messaging-telegram");
    component
        .ensure_layout()
        .expect("fixture layout should exist");
    let i18n_keys = component
        .read_i18n_keys()
        .expect("read i18n keys")
        .expect("i18n keys should exist");
    assert!(!i18n_keys.is_empty());

    for required_cbor in [
        "describe.cbor",
        "qa_default.cbor",
        "qa_setup.cbor",
        "qa_upgrade.cbor",
        "qa_remove.cbor",
        "apply_setup_config.cbor",
        "apply_upgrade_config.cbor",
        "apply_remove_config.cbor",
    ] {
        let _: JsonValue = component
            .decode_cbor_json(required_cbor)
            .expect("fixture should decode as cbor json");
    }

    for apply_file in [
        "apply_setup_config.cbor",
        "apply_upgrade_config.cbor",
        "apply_remove_config.cbor",
    ] {
        let apply = component.decode_cbor_json(apply_file).unwrap();
        assert!(
            apply.get("config").is_some(),
            "{} missing `config` field",
            apply_file
        );
    }
}
