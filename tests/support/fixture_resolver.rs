use std::path::{Path, PathBuf};

use anyhow::{Context, anyhow};
use serde::Deserialize;
use serde_json::Value as JsonValue;

#[derive(Debug, Deserialize)]
struct RegistryIndex {
    components: Vec<RegistryComponentEntry>,
}

#[derive(Debug, Deserialize)]
struct RegistryComponentEntry {
    component_id: String,
    path: String,
}

pub struct FixtureResolver {
    root: PathBuf,
}

impl FixtureResolver {
    pub fn from_root(root: impl AsRef<Path>) -> anyhow::Result<Self> {
        let root = root.as_ref().to_path_buf();
        let index_path = root.join("index.json");
        if !index_path.exists() {
            return Err(anyhow!("missing fixture index at {}", index_path.display()));
        }
        let _: RegistryIndex = serde_json::from_slice(
            &std::fs::read(&index_path)
                .with_context(|| format!("read fixture index {}", index_path.display()))?,
        )
        .with_context(|| format!("decode fixture index {}", index_path.display()))?;
        Ok(Self { root })
    }

    pub fn component(&self, component_id: &str) -> anyhow::Result<ComponentFixture> {
        let index_path = self.root.join("index.json");
        let index: RegistryIndex = serde_json::from_slice(
            &std::fs::read(&index_path)
                .with_context(|| format!("read fixture index {}", index_path.display()))?,
        )
        .with_context(|| format!("decode fixture index {}", index_path.display()))?;
        let entry = index
            .components
            .into_iter()
            .find(|entry| entry.component_id == component_id)
            .ok_or_else(|| anyhow!("component `{component_id}` not found in fixture index"))?;
        Ok(ComponentFixture {
            component_id: entry.component_id,
            root: self.root.join(entry.path),
        })
    }
}

pub struct ComponentFixture {
    component_id: String,
    root: PathBuf,
}

impl ComponentFixture {
    pub fn component_id(&self) -> &str {
        &self.component_id
    }

    pub fn decode_cbor_json(&self, file_name: &str) -> anyhow::Result<JsonValue> {
        let path = self.root.join(file_name);
        let bytes =
            std::fs::read(&path).with_context(|| format!("read fixture {}", path.display()))?;
        let value = serde_cbor::from_slice(&bytes)
            .with_context(|| format!("decode cbor fixture {}", path.display()))?;
        Ok(value)
    }

    pub fn read_i18n_keys(&self) -> anyhow::Result<Option<Vec<String>>> {
        let path = self.root.join("i18n_keys.json");
        if !path.exists() {
            return Ok(None);
        }
        let keys: Vec<String> = serde_json::from_slice(
            &std::fs::read(&path).with_context(|| format!("read {}", path.display()))?,
        )
        .with_context(|| format!("decode {}", path.display()))?;
        Ok(Some(keys))
    }

    pub fn ensure_layout(&self) -> anyhow::Result<()> {
        for required in [
            "describe.cbor",
            "qa_default.cbor",
            "qa_setup.cbor",
            "qa_upgrade.cbor",
            "qa_remove.cbor",
            "apply_setup_config.cbor",
            "apply_upgrade_config.cbor",
            "apply_remove_config.cbor",
        ] {
            let path = self.root.join(required);
            if !path.exists() {
                return Err(anyhow!("missing fixture file {}", path.display()));
            }
        }
        Ok(())
    }
}
