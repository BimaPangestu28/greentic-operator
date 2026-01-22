use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;

#[derive(Clone, Copy, Debug)]
pub enum ScanFormat {
    Text,
    Json,
    Yaml,
}

#[derive(Debug, Serialize)]
pub struct ScanReport {
    providers: BTreeMap<String, Vec<String>>,
    packs: Vec<PackEntry>,
    tenants: Vec<TenantEntry>,
}

#[derive(Debug, Serialize)]
pub struct PackEntry {
    kind: PackKind,
    path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PackKind {
    Dir,
    Gtpack,
}

#[derive(Debug, Serialize)]
pub struct TenantEntry {
    name: String,
    teams: Vec<String>,
}

pub fn scan(root: &Path) -> anyhow::Result<ScanReport> {
    let providers = scan_providers(&root.join("providers"))?;
    let packs = scan_packs(&root.join("packs"))?;
    let tenants = scan_tenants(&root.join("tenants"))?;
    Ok(ScanReport {
        providers,
        packs,
        tenants,
    })
}

pub fn render_report(report: &ScanReport, format: ScanFormat) -> anyhow::Result<()> {
    match format {
        ScanFormat::Text => render_text(report),
        ScanFormat::Json => {
            let json = serde_json::to_string_pretty(report)?;
            println!("{json}");
            Ok(())
        }
        ScanFormat::Yaml => {
            let yaml = serde_yaml_bw::to_string(report)?;
            print!("{yaml}");
            Ok(())
        }
    }
}

fn render_text(report: &ScanReport) -> anyhow::Result<()> {
    println!("Providers:");
    if report.providers.is_empty() {
        println!("  (none)");
    } else {
        for (domain, packs) in &report.providers {
            println!("  {domain}:");
            if packs.is_empty() {
                println!("    (none)");
            } else {
                for pack in packs {
                    println!("    {pack}");
                }
            }
        }
    }

    println!();
    println!("Packs:");
    if report.packs.is_empty() {
        println!("  (none)");
    } else {
        for pack in &report.packs {
            println!("  {} {}", pack.kind.label(), pack.path);
        }
    }

    println!();
    println!("Tenants:");
    if report.tenants.is_empty() {
        println!("  (none)");
    } else {
        for tenant in &report.tenants {
            if tenant.teams.is_empty() {
                println!("  {}", tenant.name);
            } else {
                println!("  {}:", tenant.name);
                for team in &tenant.teams {
                    println!("    {}", team);
                }
            }
        }
    }
    Ok(())
}

fn scan_providers(root: &Path) -> anyhow::Result<BTreeMap<String, Vec<String>>> {
    let mut providers = BTreeMap::new();
    if !root.exists() {
        return Ok(providers);
    }
    for entry in std::fs::read_dir(root)? {
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
                    packs.push(path_to_string(root, &path));
                }
            }
        }
        packs.sort();
        providers.insert(domain, packs);
    }
    Ok(providers)
}

fn scan_packs(root: &Path) -> anyhow::Result<Vec<PackEntry>> {
    let mut packs = Vec::new();
    if !root.exists() {
        return Ok(packs);
    }
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            packs.push(PackEntry {
                kind: PackKind::Dir,
                path: path_to_string(root, &path),
            });
        } else if entry.file_type()?.is_file()
            && path.extension().and_then(|ext| ext.to_str()) == Some("gtpack")
        {
            packs.push(PackEntry {
                kind: PackKind::Gtpack,
                path: path_to_string(root, &path),
            });
        }
    }
    packs.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(packs)
}

fn scan_tenants(root: &Path) -> anyhow::Result<Vec<TenantEntry>> {
    let mut tenants = Vec::new();
    if !root.exists() {
        return Ok(tenants);
    }
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let teams_dir = entry.path().join("teams");
        let mut teams = Vec::new();
        if teams_dir.exists() {
            for team_entry in std::fs::read_dir(teams_dir)? {
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

fn path_to_string(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

impl PackKind {
    fn label(&self) -> &'static str {
        match self {
            PackKind::Dir => "dir",
            PackKind::Gtpack => "gtpack",
        }
    }
}
