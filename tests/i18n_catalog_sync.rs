use std::collections::BTreeSet;
use std::path::PathBuf;

fn load_keys(path: &PathBuf) -> BTreeSet<String> {
    let raw = std::fs::read_to_string(path).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", path.display());
    });
    let value: serde_json::Value = serde_json::from_str(&raw).unwrap_or_else(|err| {
        panic!("failed to parse {}: {err}", path.display());
    });
    value
        .as_object()
        .unwrap_or_else(|| panic!("{} is not a JSON object", path.display()))
        .keys()
        .cloned()
        .collect()
}

fn assert_catalog_sync(dir: &str, base_file: &str) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(dir);
    let base = root.join(base_file);
    let base_keys = load_keys(&base);

    for entry in std::fs::read_dir(&root).expect("read catalog dir") {
        let entry = entry.expect("read dir entry");
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let keys = load_keys(&path);
        assert_eq!(
            keys,
            base_keys,
            "translation key mismatch for {}",
            path.display()
        );
    }
}

#[test]
fn wizard_catalogs_keep_same_key_set() {
    assert_catalog_sync("i18n/operator_wizard", "en.json");
}

#[test]
fn cli_catalogs_keep_same_key_set() {
    assert_catalog_sync("i18n/operator_cli", "en.json");
}
