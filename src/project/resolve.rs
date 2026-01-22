use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;

const VERSION: &str = "1";
const DEFAULT_POLICY: &str = "forbidden";
const ENV_PASSTHROUGH: [&str; 3] = [
    "OTEL_EXPORTER_OTLP_ENDPOINT",
    "OTEL_RESOURCE_ATTRIBUTES",
    "RUST_LOG",
];

#[derive(Debug, Serialize)]
struct ResolvedManifest {
    version: String,
    tenant: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    team: Option<String>,
    project_root: String,
    providers: BTreeMap<String, Vec<String>>,
    packs: Vec<String>,
    env_passthrough: Vec<String>,
    policy: PolicySection,
}

#[derive(Debug, Serialize)]
struct PolicySection {
    source: PolicySource,
    default: String,
}

#[derive(Debug, Serialize)]
struct PolicySource {
    tenant_gmap: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    team_gmap: Option<String>,
}

#[derive(Debug)]
struct TenantEntry {
    name: String,
    teams: Vec<String>,
}

pub fn resolve(root: &Path) -> anyhow::Result<()> {
    let providers = scan_providers(root)?;
    let packs = scan_packs(root)?;
    let tenants = scan_tenants(root)?;
    let project_root = root.to_string_lossy().to_string();

    let resolved_dir = root.join("state").join("resolved");
    std::fs::create_dir_all(&resolved_dir)?;

    for tenant in tenants {
        if tenant.teams.is_empty() {
            let manifest =
                build_manifest(&tenant.name, None, &project_root, &providers, &packs, root);
            let filename = resolved_dir.join(format!("{}.yaml", tenant.name));
            write_manifest(&filename, &manifest)?;
        } else {
            for team in tenant.teams {
                let manifest = build_manifest(
                    &tenant.name,
                    Some(&team),
                    &project_root,
                    &providers,
                    &packs,
                    root,
                );
                let filename = resolved_dir.join(format!("{}.{}.yaml", tenant.name, team));
                write_manifest(&filename, &manifest)?;
            }
        }
    }

    Ok(())
}

fn build_manifest(
    tenant: &str,
    team: Option<&str>,
    project_root: &str,
    providers: &BTreeMap<String, Vec<String>>,
    packs: &[String],
    root: &Path,
) -> ResolvedManifest {
    let tenant_gmap = relative_path(root, &root.join("tenants").join(tenant).join("tenant.gmap"));
    let team_gmap = team.map(|team| {
        relative_path(
            root,
            &root
                .join("tenants")
                .join(tenant)
                .join("teams")
                .join(team)
                .join("team.gmap"),
        )
    });

    ResolvedManifest {
        version: VERSION.to_string(),
        tenant: tenant.to_string(),
        team: team.map(|value| value.to_string()),
        project_root: project_root.to_string(),
        providers: providers.clone(),
        packs: packs.to_vec(),
        env_passthrough: ENV_PASSTHROUGH
            .iter()
            .map(|value| value.to_string())
            .collect(),
        policy: PolicySection {
            source: PolicySource {
                tenant_gmap,
                team_gmap,
            },
            default: DEFAULT_POLICY.to_string(),
        },
    }
}

fn write_manifest(path: &Path, manifest: &ResolvedManifest) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let yaml = serde_yaml_bw::to_string(manifest)?;
    std::fs::write(path, yaml)?;
    Ok(())
}

fn scan_providers(root: &Path) -> anyhow::Result<BTreeMap<String, Vec<String>>> {
    let mut providers = BTreeMap::new();
    let providers_root = root.join("providers");
    if !providers_root.exists() {
        return Ok(providers);
    }
    for entry in std::fs::read_dir(&providers_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let domain = entry.file_name().to_string_lossy().to_string();
        let mut packs = Vec::new();
        for pack in std::fs::read_dir(entry.path())? {
            let pack = pack?;
            if pack.file_type()?.is_file() {
                let path = pack.path();
                if path.extension().and_then(|ext| ext.to_str()) == Some("gtpack") {
                    packs.push(relative_path(root, &path));
                }
            }
        }
        packs.sort();
        providers.insert(domain, packs);
    }
    Ok(providers)
}

fn scan_packs(root: &Path) -> anyhow::Result<Vec<String>> {
    let mut packs = Vec::new();
    let packs_root = root.join("packs");
    if !packs_root.exists() {
        return Ok(packs);
    }
    for entry in std::fs::read_dir(&packs_root)? {
        let entry = entry?;
        let path = entry.path();
        let is_pack_dir = entry.file_type()?.is_dir();
        let is_gtpack = entry.file_type()?.is_file()
            && path.extension().and_then(|ext| ext.to_str()) == Some("gtpack");
        if is_pack_dir || is_gtpack {
            packs.push(relative_path(root, &path));
        }
    }
    packs.sort();
    Ok(packs)
}

fn scan_tenants(root: &Path) -> anyhow::Result<Vec<TenantEntry>> {
    let tenants_root = root.join("tenants");
    let mut tenants = Vec::new();
    if !tenants_root.exists() {
        return Ok(tenants);
    }
    for entry in std::fs::read_dir(&tenants_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let teams_root = entry.path().join("teams");
        let mut teams = Vec::new();
        if teams_root.exists() {
            for team_entry in std::fs::read_dir(teams_root)? {
                let team_entry = team_entry?;
                if team_entry.file_type()?.is_dir() {
                    teams.push(team_entry.file_name().to_string_lossy().to_string());
                }
            }
        }
        teams.sort();
        tenants.push(TenantEntry { name, teams });
    }
    tenants.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(tenants)
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}
