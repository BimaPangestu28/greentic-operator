use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use serde_cbor::from_slice;

#[derive(Debug, Clone)]
pub struct DemoPack {
    pub pack_id: String,
    pub pack_path: PathBuf,
    pub entry_flows: Vec<String>,
    pub default_flow_source: EntryFlowSource,
}

#[derive(Clone, Copy, Debug)]
pub enum EntryFlowSource {
    MetaEntryFlows,
    FlowsList,
}

impl EntryFlowSource {
    fn description(self) -> &'static str {
        match self {
            EntryFlowSource::MetaEntryFlows => "meta.entry_flows",
            EntryFlowSource::FlowsList => "manifest.flows[*].id/entrypoints",
        }
    }
}

impl DemoPack {
    pub fn select_flow(&self, requested: Option<&str>) -> Result<String> {
        if self.entry_flows.is_empty() {
            return Err(anyhow!(
                "default flow not declared ({}) and no flows available; available flows: []",
                self.default_flow_source.description()
            ));
        }
        if let Some(flow) = requested {
            if self.entry_flows.iter().any(|candidate| candidate == flow) {
                return Ok(flow.to_string());
            }
            return Err(anyhow!(
                "flow `{flow}` not declared via {}; available flows: {}",
                self.default_flow_source.description(),
                self.entry_flows.join(", ")
            ));
        }
        Ok(self.entry_flows[0].clone())
    }
}

pub fn resolve_pack(packs_dir: &Path, pack_name: &str) -> Result<DemoPack> {
    let pack_path = packs_dir.join(pack_name);
    if !pack_path.exists() {
        return Err(anyhow!(
            "pack {pack_name} not found under {}",
            packs_dir.display()
        ));
    }
    let manifest_path = pack_path.join("manifest.cbor");
    let bytes = fs::read(&manifest_path).with_context(|| {
        format!(
            "unable to read manifest.cbor for pack {}",
            manifest_path.display()
        )
    })?;
    let manifest: Manifest = from_slice(&bytes).with_context(|| {
        format!(
            "failed to decode manifest.cbor for pack {}",
            manifest_path.display()
        )
    })?;
    let pack_id = manifest.pack_id(pack_name);
    let (entry_flows, source) = manifest.entry_flows();
    Ok(DemoPack {
        pack_id,
        pack_path,
        entry_flows,
        default_flow_source: source,
    })
}

#[derive(Debug, Deserialize)]
struct Manifest {
    #[serde(default)]
    pack_id: Option<String>,
    #[serde(default)]
    meta: Option<ManifestMeta>,
    #[serde(default)]
    flows: Vec<ManifestFlow>,
}

#[derive(Debug, Deserialize)]
struct ManifestMeta {
    #[serde(default)]
    pack_id: Option<String>,
    #[serde(default)]
    entry_flows: Option<EntryFlows>,
}

#[derive(Debug, Deserialize)]
struct ManifestFlow {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    entrypoints: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum EntryFlows {
    List(Vec<String>),
    Map(BTreeMap<String, String>),
}

impl EntryFlows {
    fn values(&self) -> Vec<String> {
        match self {
            EntryFlows::List(list) => list.clone(),
            EntryFlows::Map(map) => map.values().cloned().collect(),
        }
    }
}

impl Manifest {
    fn pack_id(&self, fallback: &str) -> String {
        if let Some(meta) = &self.meta {
            if let Some(pack_id) = &meta.pack_id {
                if !pack_id.is_empty() {
                    return pack_id.clone();
                }
            }
        }
        if let Some(pack_id) = &self.pack_id {
            if !pack_id.is_empty() {
                return pack_id.clone();
            }
        }
        fallback.to_string()
    }

    fn entry_flows(&self) -> (Vec<String>, EntryFlowSource) {
        if let Some(meta) = &self.meta {
            if let Some(entry_flows) = &meta.entry_flows {
                let values = entry_flows
                    .values()
                    .into_iter()
                    .filter(|value| !value.is_empty())
                    .collect::<Vec<_>>();
                if !values.is_empty() {
                    return (values, EntryFlowSource::MetaEntryFlows);
                }
                return (Vec::new(), EntryFlowSource::MetaEntryFlows);
            }
        }
        let mut flows = Vec::new();
        for manifest_flow in &self.flows {
            if let Some(id) = manifest_flow.id.as_ref() {
                if !id.is_empty() {
                    flows.push(id.clone());
                }
            }
            for entry in &manifest_flow.entrypoints {
                if !entry.is_empty() {
                    flows.push(entry.clone());
                }
            }
        }
        (flows, EntryFlowSource::FlowsList)
    }
}
