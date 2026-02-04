use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};

use crate::domains;

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
    let meta = domains::read_pack_meta(&pack_path)
        .with_context(|| format!("failed to read manifest for pack {}", pack_path.display()))?;
    Ok(DemoPack {
        pack_id: meta.pack_id.clone(),
        pack_path,
        entry_flows: meta.entry_flows.clone(),
        default_flow_source: EntryFlowSource::MetaEntryFlows,
    })
}
