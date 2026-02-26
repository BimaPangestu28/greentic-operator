use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct PackBinding {
    pub pack_id: String,
    pub pack_ref: String,
    pub pack_locator: Option<String>,
    pub flows: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TenantBindings {
    pub tenant: String,
    pub packs: Vec<PackBinding>,
    pub env_passthrough: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GtBindFile {
    tenant: String,
    pack_id: String,
    pack_ref: String,
    #[serde(default)]
    pack_locator: Option<String>,
    #[serde(default)]
    flows: Vec<GtBindFlow>,
    #[serde(default)]
    env_passthrough: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GtBindFlow {
    id: String,
}

pub fn collect_gtbind_paths(paths: &[PathBuf], dirs: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut resolved = Vec::new();
    for path in paths {
        if path.is_dir() {
            resolved.extend(scan_dir(path)?);
        } else if path.is_file() {
            resolved.push(path.to_path_buf());
        } else {
            bail!("bindings path does not exist: {}", path.display());
        }
    }
    for dir in dirs {
        if !dir.is_dir() {
            bail!("bindings dir does not exist: {}", dir.display());
        }
        resolved.extend(scan_dir(dir)?);
    }
    resolved.sort();
    resolved.dedup();
    Ok(resolved)
}

pub fn load_gtbinds(paths: &[PathBuf]) -> Result<HashMap<String, TenantBindings>> {
    let mut tenants: HashMap<String, TenantBindings> = HashMap::new();
    for path in paths {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read gtbind {}", path.display()))?;
        let raw: GtBindFile = serde_yaml_bw::from_str(&content)
            .with_context(|| format!("failed to parse gtbind {}", path.display()))?;
        if raw.pack_id.trim().is_empty() {
            bail!("gtbind {} missing pack_id", path.display());
        }
        if raw.pack_ref.trim().is_empty() {
            bail!("gtbind {} missing pack_ref", path.display());
        }
        if raw.tenant.trim().is_empty() {
            bail!("gtbind {} missing tenant", path.display());
        }
        let flows = raw
            .flows
            .into_iter()
            .map(|flow| flow.id)
            .filter(|id| !id.trim().is_empty())
            .collect::<Vec<_>>();
        let pack = PackBinding {
            pack_id: raw.pack_id,
            pack_ref: raw.pack_ref,
            pack_locator: raw.pack_locator,
            flows,
        };
        let entry = tenants
            .entry(raw.tenant.clone())
            .or_insert_with(|| TenantBindings {
                tenant: raw.tenant.clone(),
                packs: Vec::new(),
                env_passthrough: Vec::new(),
            });
        merge_pack(entry, pack)?;
        merge_env(entry, raw.env_passthrough);
    }
    Ok(tenants)
}

fn scan_dir(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("gtbind") {
            entries.push(path);
        }
    }
    Ok(entries)
}

fn merge_pack(tenant: &mut TenantBindings, pack: PackBinding) -> Result<()> {
    if let Some(existing) = tenant
        .packs
        .iter_mut()
        .find(|entry| entry.pack_id == pack.pack_id)
    {
        if existing.pack_ref != pack.pack_ref {
            bail!(
                "pack_ref mismatch for tenant {} pack {}",
                tenant.tenant,
                pack.pack_id
            );
        }
        match (&existing.pack_locator, &pack.pack_locator) {
            (Some(existing), Some(incoming)) if existing != incoming => {
                bail!(
                    "pack_locator mismatch for tenant {} pack {}",
                    tenant.tenant,
                    pack.pack_id
                );
            }
            (None, Some(incoming)) => {
                existing.pack_locator = Some(incoming.clone());
            }
            _ => {}
        }
        let mut flows = HashSet::new();
        flows.extend(existing.flows.iter().cloned());
        flows.extend(pack.flows);
        existing.flows = flows.into_iter().collect();
        existing.flows.sort();
        return Ok(());
    }
    tenant.packs.push(pack);
    tenant.packs.sort_by(|a, b| a.pack_id.cmp(&b.pack_id));
    Ok(())
}

fn merge_env(tenant: &mut TenantBindings, envs: Vec<String>) {
    let mut merged = HashSet::new();
    merged.extend(tenant.env_passthrough.iter().cloned());
    merged.extend(envs);
    tenant.env_passthrough = merged.into_iter().collect();
    tenant.env_passthrough.sort();
}
