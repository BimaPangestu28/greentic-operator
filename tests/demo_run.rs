use std::{fs, path::Path};

use anyhow::Result;
use serde_json::json;
use tempfile::{NamedTempFile, tempdir};

use greentic_operator::demo::input as demo_input;
use greentic_operator::demo::pack_resolve;

#[test]
fn resolve_pack_uses_entry_flows_as_default() -> Result<()> {
    let root = tempdir()?;
    let pack_dir = root.path().join("demo-pack");
    fs::create_dir_all(&pack_dir)?;
    write_manifest(&pack_dir, "demo-pack-id", Some(&["setup"]), &[])?;
    let pack = pack_resolve::resolve_pack(root.path(), "demo-pack")?;
    assert_eq!(pack.pack_id, "demo-pack-id");
    assert_eq!(pack.entry_flows, vec!["setup"]);
    assert_eq!(pack.select_flow(None)?, "setup".to_string());
    Ok(())
}

#[test]
fn resolve_pack_default_flow_missing_mentions_field() -> Result<()> {
    let root = tempdir()?;
    let pack_dir = root.path().join("demo-pack");
    fs::create_dir_all(&pack_dir)?;
    write_manifest(&pack_dir, "demo-pack-id", Some(&[]), &[])?;
    let pack = pack_resolve::resolve_pack(root.path(), "demo-pack")?;
    let err = pack.select_flow(None).unwrap_err();
    let text = err.to_string();
    assert!(
        text.contains("meta.entry_flows"),
        "error lacked field hint: {text}"
    );
    assert!(
        text.contains("available flows:"),
        "error lacked available flows list: {text}"
    );
    Ok(())
}

#[test]
fn parse_input_json_yaml_and_file() -> Result<()> {
    let inline_json = r#"{"foo": "bar"}"#;
    let parsed_json = demo_input::parse_input(inline_json)?;
    assert!(matches!(
        parsed_json.source,
        demo_input::InputSource::Inline(demo_input::InputEncoding::Json)
    ));
    assert_eq!(parsed_json.value["foo"], json!("bar"));

    let inline_yaml = r#"foo: 42"#;
    let parsed_yaml = demo_input::parse_input(inline_yaml)?;
    assert!(matches!(
        parsed_yaml.source,
        demo_input::InputSource::Inline(demo_input::InputEncoding::Yaml)
    ));
    assert_eq!(parsed_yaml.value["foo"], json!(42));

    let file = NamedTempFile::new()?;
    fs::write(file.path(), r#"{"file": true}"#)?;
    let parsed_file = demo_input::parse_input(&format!("@{}", file.path().display()))?;
    if let demo_input::InputSource::File { path, encoding } = parsed_file.source {
        assert_eq!(path, file.path());
        assert_eq!(encoding, demo_input::InputEncoding::Json);
    } else {
        panic!("expected file input source");
    }
    assert_eq!(parsed_file.value["file"], json!(true));
    Ok(())
}

fn write_manifest(
    pack_dir: &Path,
    pack_id: &str,
    entry_flows: Option<&[&str]>,
    flows: &[&str],
) -> Result<()> {
    let mut meta = serde_json::Map::new();
    meta.insert("pack_id".to_string(), json!(pack_id));
    if let Some(entry_flows) = entry_flows {
        meta.insert("entry_flows".to_string(), json!(entry_flows));
    }
    let manifest = json!({
        "schema_version": "greentic.pack-v1",
        "pack_id": pack_id,
        "meta": meta,
        "flows": flows.iter().map(|id| json!({ "id": id })).collect::<Vec<_>>(),
    });
    let bytes = serde_cbor::to_vec(&manifest)?;
    fs::write(pack_dir.join("manifest.cbor"), bytes)?;
    Ok(())
}
