use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn en_keys(path: &str) -> BTreeSet<String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path);
    let raw = std::fs::read_to_string(root).expect("read en catalog");
    let value: serde_json::Value = serde_json::from_str(&raw).expect("parse en catalog");
    value
        .as_object()
        .expect("catalog object")
        .keys()
        .cloned()
        .collect()
}

fn rust_files_under(path: &str) -> Vec<PathBuf> {
    fn walk(dir: &PathBuf, out: &mut Vec<PathBuf>) {
        let entries = fs::read_dir(dir).unwrap_or_else(|err| panic!("read_dir {dir:?}: {err}"));
        for entry in entries {
            let entry = entry.unwrap_or_else(|err| panic!("read_dir entry {dir:?}: {err}"));
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }

    let mut files = Vec::new();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path);
    walk(&root, &mut files);
    files
}

#[test]
fn cli_i18n_keys_used_in_code_exist_in_catalog() {
    let keys = en_keys("i18n/operator_cli/en.json");
    for path in rust_files_under("src") {
        let source_file = path
            .strip_prefix(PathBuf::from(env!("CARGO_MANIFEST_DIR")))
            .unwrap_or(&path)
            .display()
            .to_string();
        let src = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
        for pattern in ["operator_i18n::tr(\"", "operator_i18n::trf(\""] {
            for cap in src.match_indices(pattern) {
                let rest = &src[cap.0 + pattern.len()..];
                if let Some(end) = rest.find('"') {
                    let key = &rest[..end];
                    assert!(
                        keys.contains(key),
                        "missing i18n key in operator_cli/en.json from {source_file}: {key}"
                    );
                }
            }
        }
    }
}

#[test]
fn wizard_i18n_keys_exist_in_wizard_catalog() {
    let keys = en_keys("i18n/operator_wizard/en.json");
    let src = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/wizard_spec_builder.rs"),
    )
    .expect("read wizard_spec_builder.rs");
    for cap in src.match_indices("\"key\": \"wizard.") {
        let rest = &src[cap.0 + "\"key\": \"".len()..];
        if let Some(end) = rest.find('"') {
            let key = &rest[..end];
            assert!(
                keys.contains(key),
                "missing wizard i18n key in operator_wizard/en.json: {key}"
            );
        }
    }
}
