use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, anyhow};
use serde::{Deserialize, Serialize};

use crate::gmap::{self, Policy};
use crate::project;

#[derive(Clone, Debug, Serialize)]
pub struct QaQuestion {
    pub id: String,
    pub title: String,
    pub required: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct QaSpec {
    pub mode: String,
    pub questions: Vec<QaQuestion>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WizardMode {
    Create,
    Update,
    Remove,
}

impl WizardMode {
    pub fn as_str(self) -> &'static str {
        match self {
            WizardMode::Create => "create",
            WizardMode::Update => "update",
            WizardMode::Remove => "remove",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct WizardPlan {
    pub mode: String,
    pub dry_run: bool,
    pub bundle: PathBuf,
    pub steps: Vec<WizardPlanStep>,
    pub metadata: WizardPlanMetadata,
}

#[derive(Clone, Debug, Serialize)]
pub struct WizardPlanMetadata {
    pub pack_refs: Vec<String>,
    pub tenants: Vec<TenantSelection>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WizardPlanStep {
    pub kind: WizardStepKind,
    pub description: String,
    pub details: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WizardStepKind {
    ResolvePacks,
    CreateBundle,
    AddPacksToBundle,
    ApplyPackSetup,
    WriteGmapRules,
    RunResolver,
    CopyResolvedManifest,
    ValidateBundle,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackListing {
    pub id: String,
    pub label: String,
    pub reference: String,
}

pub trait CatalogSource {
    fn list(&self) -> Vec<PackListing>;
}

#[derive(Clone, Debug, Default)]
pub struct StaticCatalogSource;

impl CatalogSource for StaticCatalogSource {
    fn list(&self) -> Vec<PackListing> {
        // Listing only; fetching is delegated to distributor client in execution.
        vec![
            PackListing {
                id: "messaging-telegram".to_string(),
                label: "Messaging Telegram".to_string(),
                reference: "repo://messaging/providers/messaging-telegram@latest".to_string(),
            },
            PackListing {
                id: "messaging-slack".to_string(),
                label: "Messaging Slack".to_string(),
                reference: "repo://messaging/providers/messaging-slack@latest".to_string(),
            },
        ]
    }
}

pub fn load_catalog_from_file(path: &Path) -> anyhow::Result<Vec<PackListing>> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("read catalog file {}", path.display()))?;
    let parsed: Vec<PackListing> = serde_json::from_str(&raw)
        .or_else(|_| serde_yaml_bw::from_str(&raw))
        .with_context(|| format!("parse catalog file {}", path.display()))?;
    Ok(parsed)
}

#[derive(Clone, Debug, Serialize)]
pub struct TenantSelection {
    pub tenant: String,
    pub team: Option<String>,
    pub allow_paths: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct WizardCreateRequest {
    pub bundle: PathBuf,
    pub pack_refs: Vec<String>,
    pub tenants: Vec<TenantSelection>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ResolvedPackInfo {
    pub source_ref: String,
    pub mapped_ref: String,
    pub resolved_digest: String,
    pub pack_id: String,
    pub entry_flows: Vec<String>,
    pub cached_path: PathBuf,
    pub output_path: PathBuf,
}

#[derive(Clone, Debug, Serialize)]
pub struct WizardExecutionReport {
    pub bundle: PathBuf,
    pub resolved_packs: Vec<ResolvedPackInfo>,
    pub resolved_manifests: Vec<PathBuf>,
}

pub fn spec(mode: WizardMode) -> QaSpec {
    QaSpec {
        mode: mode.as_str().to_string(),
        questions: vec![
            QaQuestion {
                id: "operator.bundle.path".to_string(),
                title: "Bundle output path".to_string(),
                required: true,
            },
            QaQuestion {
                id: "operator.packs.refs".to_string(),
                title: "Pack refs (catalog + custom)".to_string(),
                required: false,
            },
            QaQuestion {
                id: "operator.tenants".to_string(),
                title: "Tenants and optional teams".to_string(),
                required: true,
            },
            QaQuestion {
                id: "operator.allow.paths".to_string(),
                title: "Allow rules as PACK[/FLOW[/NODE]]".to_string(),
                required: false,
            },
        ],
    }
}

pub fn apply_create(request: &WizardCreateRequest, dry_run: bool) -> anyhow::Result<WizardPlan> {
    if request.tenants.is_empty() {
        return Err(anyhow!("at least one tenant selection is required"));
    }

    let mut pack_refs = request
        .pack_refs
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    pack_refs.sort();
    pack_refs.dedup();

    let mut tenants = request.tenants.clone();
    for tenant in &mut tenants {
        tenant.allow_paths.sort();
        tenant.allow_paths.dedup();
    }
    tenants.sort_by(|a, b| {
        a.tenant
            .cmp(&b.tenant)
            .then_with(|| a.team.cmp(&b.team))
            .then_with(|| a.allow_paths.cmp(&b.allow_paths))
    });

    let mut steps = Vec::new();
    if !pack_refs.is_empty() {
        steps.push(step(
            WizardStepKind::ResolvePacks,
            "Resolve selected pack refs via distributor client",
            [("count", pack_refs.len().to_string())],
        ));
    }
    steps.push(step(
        WizardStepKind::CreateBundle,
        "Create demo bundle scaffold using existing conventions",
        [("bundle", request.bundle.display().to_string())],
    ));
    if !pack_refs.is_empty() {
        steps.push(step(
            WizardStepKind::AddPacksToBundle,
            "Copy fetched packs into bundle/packs",
            [("count", pack_refs.len().to_string())],
        ));
        steps.push(step(
            WizardStepKind::ApplyPackSetup,
            "Apply pack-declared setup outputs through internal setup hooks",
            [("status", "planned".to_string())],
        ));
    }
    steps.push(step(
        WizardStepKind::WriteGmapRules,
        "Write tenant/team allow rules to gmap",
        [("targets", tenants.len().to_string())],
    ));
    steps.push(step(
        WizardStepKind::RunResolver,
        "Run resolver pipeline (same as demo allow)",
        [("resolver", "project::sync_project".to_string())],
    ));
    steps.push(step(
        WizardStepKind::CopyResolvedManifest,
        "Copy state/resolved manifests into resolved/ for demo start",
        [("targets", tenants.len().to_string())],
    ));
    steps.push(step(
        WizardStepKind::ValidateBundle,
        "Validate bundle is loadable by internal demo pipeline",
        [("check", "resolved manifests present".to_string())],
    ));

    Ok(WizardPlan {
        mode: "create".to_string(),
        dry_run,
        bundle: request.bundle.clone(),
        steps,
        metadata: WizardPlanMetadata { pack_refs, tenants },
    })
}

pub fn apply_update(request: &WizardCreateRequest, dry_run: bool) -> anyhow::Result<WizardPlan> {
    let mut plan = apply_create(request, dry_run)?;
    plan.mode = WizardMode::Update.as_str().to_string();
    plan.steps
        .retain(|step| step.kind != WizardStepKind::CreateBundle);
    if !plan.metadata.pack_refs.is_empty() {
        plan.steps
            .retain(|step| step.kind != WizardStepKind::ResolvePacks);
        plan.steps.insert(
            0,
            step(
                WizardStepKind::ResolvePacks,
                "Resolve selected pack refs via distributor client",
                [("count", plan.metadata.pack_refs.len().to_string())],
            ),
        );
    }
    plan.steps.insert(
        0,
        step(
            WizardStepKind::ValidateBundle,
            "Validate target bundle exists before update",
            [("mode", "update".to_string())],
        ),
    );
    Ok(plan)
}

pub fn apply_remove(request: &WizardCreateRequest, dry_run: bool) -> anyhow::Result<WizardPlan> {
    if request.tenants.is_empty() {
        return Err(anyhow!("at least one tenant selection is required"));
    }

    let mut tenants = request.tenants.clone();
    for tenant in &mut tenants {
        tenant.allow_paths.sort();
        tenant.allow_paths.dedup();
    }
    tenants.sort_by(|a, b| {
        a.tenant
            .cmp(&b.tenant)
            .then_with(|| a.team.cmp(&b.team))
            .then_with(|| a.allow_paths.cmp(&b.allow_paths))
    });

    let steps = vec![
        step(
            WizardStepKind::ValidateBundle,
            "Validate target bundle exists before remove",
            [("mode", "remove".to_string())],
        ),
        step(
            WizardStepKind::WriteGmapRules,
            "Write forbidden tenant/team rules to gmap",
            [("targets", tenants.len().to_string())],
        ),
        step(
            WizardStepKind::RunResolver,
            "Run resolver pipeline (same as demo forbid)",
            [("resolver", "project::sync_project".to_string())],
        ),
        step(
            WizardStepKind::CopyResolvedManifest,
            "Copy state/resolved manifests into resolved/ for demo start",
            [("targets", tenants.len().to_string())],
        ),
        step(
            WizardStepKind::ValidateBundle,
            "Validate bundle is loadable by internal demo pipeline",
            [("check", "resolved manifests present".to_string())],
        ),
    ];

    Ok(WizardPlan {
        mode: WizardMode::Remove.as_str().to_string(),
        dry_run,
        bundle: request.bundle.clone(),
        steps,
        metadata: WizardPlanMetadata {
            pack_refs: Vec::new(),
            tenants,
        },
    })
}

pub fn apply(
    mode: WizardMode,
    request: &WizardCreateRequest,
    dry_run: bool,
) -> anyhow::Result<WizardPlan> {
    match mode {
        WizardMode::Create => apply_create(request, dry_run),
        WizardMode::Update => apply_update(request, dry_run),
        WizardMode::Remove => apply_remove(request, dry_run),
    }
}

pub fn execute_plan(
    mode: WizardMode,
    plan: &WizardPlan,
    offline: bool,
) -> anyhow::Result<WizardExecutionReport> {
    match mode {
        WizardMode::Create => execute_create_plan(plan, offline),
        WizardMode::Update => execute_update_plan(plan, offline),
        WizardMode::Remove => execute_remove_plan(plan),
    }
}

fn sync_allow_and_resolved(
    bundle: &Path,
    tenants: &[TenantSelection],
    policy: Policy,
) -> anyhow::Result<Vec<PathBuf>> {
    let mut target_manifests = BTreeSet::new();
    for tenant in tenants {
        ensure_tenant_and_team(bundle, tenant)?;
        let effective_team = if let Some(team) = tenant.team.as_deref() {
            if team.is_empty() {
                None
            } else {
                Some(team.to_string())
            }
        } else if bundle
            .join("tenants")
            .join(&tenant.tenant)
            .join("teams")
            .join("default")
            .exists()
        {
            Some("default".to_string())
        } else {
            None
        };
        for path in &tenant.allow_paths {
            if path.trim().is_empty() {
                continue;
            }
            let gmap_path =
                demo_bundle_gmap_path(bundle, &tenant.tenant, effective_team.as_deref());
            gmap::upsert_policy(&gmap_path, path, policy.clone())?;
        }
        target_manifests.insert(resolved_manifest_filename(
            &tenant.tenant,
            effective_team.as_deref(),
        ));
    }

    project::sync_project(bundle)?;

    let mut copied = Vec::new();
    for filename in target_manifests {
        let src = bundle.join("state").join("resolved").join(&filename);
        if !src.exists() {
            return Err(anyhow!("resolved manifest not found at {}", src.display()));
        }
        let dst = bundle.join("resolved").join(&filename);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&src, &dst)?;
        copied.push(dst);
    }
    Ok(copied)
}

fn validate_bundle_exists(bundle: &Path) -> anyhow::Result<()> {
    if !bundle.exists() {
        return Err(anyhow!("bundle path {} does not exist", bundle.display()));
    }
    if !bundle.join("greentic.demo.yaml").exists() {
        return Err(anyhow!(
            "bundle {} missing greentic.demo.yaml",
            bundle.display()
        ));
    }
    Ok(())
}

pub fn print_plan_summary(plan: &WizardPlan) {
    println!("wizard plan: mode={} dry_run={}", plan.mode, plan.dry_run);
    println!("bundle: {}", plan.bundle.display());
    for (index, step) in plan.steps.iter().enumerate() {
        println!("{}. {:?}: {}", index + 1, step.kind, step.description);
    }
}

pub fn confirm_execute() -> anyhow::Result<bool> {
    print!("Execute this plan? [y/N]: ");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    let value = line.trim().to_ascii_lowercase();
    Ok(matches!(value.as_str(), "y" | "yes"))
}

pub fn execute_create_plan(
    plan: &WizardPlan,
    offline: bool,
) -> anyhow::Result<WizardExecutionReport> {
    if plan.mode != WizardMode::Create.as_str() {
        return Err(anyhow!("unsupported wizard mode: {}", plan.mode));
    }

    if plan.bundle.exists() {
        return Err(anyhow!(
            "bundle path {} already exists",
            plan.bundle.display()
        ));
    }

    create_demo_bundle_structure(&plan.bundle)?;

    let mut resolved_packs = Vec::new();
    if !plan.metadata.pack_refs.is_empty() {
        let resolved = resolve_pack_refs(&plan.metadata.pack_refs, offline)
            .context("resolve pack refs via distributor-client")?;
        for item in resolved {
            copy_pack_into_bundle(&plan.bundle, &item)?;
            resolved_packs.push(item);
        }
    }

    let copied = sync_allow_and_resolved(&plan.bundle, &plan.metadata.tenants, Policy::Public)?;

    Ok(WizardExecutionReport {
        bundle: plan.bundle.clone(),
        resolved_packs,
        resolved_manifests: copied,
    })
}

pub fn execute_update_plan(
    plan: &WizardPlan,
    offline: bool,
) -> anyhow::Result<WizardExecutionReport> {
    if plan.mode != WizardMode::Update.as_str() {
        return Err(anyhow!("unsupported wizard mode: {}", plan.mode));
    }
    validate_bundle_exists(&plan.bundle)?;

    let mut resolved_packs = Vec::new();
    if !plan.metadata.pack_refs.is_empty() {
        let resolved = resolve_pack_refs(&plan.metadata.pack_refs, offline)
            .context("resolve pack refs via distributor-client")?;
        for item in resolved {
            copy_pack_into_bundle(&plan.bundle, &item)?;
            resolved_packs.push(item);
        }
    }

    let copied = sync_allow_and_resolved(&plan.bundle, &plan.metadata.tenants, Policy::Public)?;
    Ok(WizardExecutionReport {
        bundle: plan.bundle.clone(),
        resolved_packs,
        resolved_manifests: copied,
    })
}

pub fn execute_remove_plan(plan: &WizardPlan) -> anyhow::Result<WizardExecutionReport> {
    if plan.mode != WizardMode::Remove.as_str() {
        return Err(anyhow!("unsupported wizard mode: {}", plan.mode));
    }
    validate_bundle_exists(&plan.bundle)?;
    let copied = sync_allow_and_resolved(&plan.bundle, &plan.metadata.tenants, Policy::Forbidden)?;
    Ok(WizardExecutionReport {
        bundle: plan.bundle.clone(),
        resolved_packs: Vec::new(),
        resolved_manifests: copied,
    })
}

fn step<const N: usize>(
    kind: WizardStepKind,
    description: &str,
    details: [(&str, String); N],
) -> WizardPlanStep {
    let mut map = BTreeMap::new();
    for (key, value) in details {
        map.insert(key.to_string(), value);
    }
    WizardPlanStep {
        kind,
        description: description.to_string(),
        details: map,
    }
}

fn create_demo_bundle_structure(root: &Path) -> anyhow::Result<()> {
    let directories = [
        "",
        "providers",
        "providers/messaging",
        "providers/events",
        "providers/secrets",
        "packs",
        "resolved",
        "state",
        "state/resolved",
        "state/runs",
        "state/pids",
        "state/logs",
        "state/runtime",
        "state/doctor",
        "tenants",
        "tenants/default",
        "tenants/default/teams",
        "tenants/demo",
        "tenants/demo/teams",
        "tenants/demo/teams/default",
        "logs",
    ];
    for directory in directories {
        std::fs::create_dir_all(root.join(directory))?;
    }
    write_if_missing(
        &root.join("greentic.demo.yaml"),
        "version: \"1\"\nproject_root: \"./\"\n",
    )?;
    write_if_missing(
        &root.join("tenants").join("default").join("tenant.gmap"),
        "_ = forbidden\n",
    )?;
    write_if_missing(
        &root.join("tenants").join("demo").join("tenant.gmap"),
        "_ = forbidden\n",
    )?;
    write_if_missing(
        &root
            .join("tenants")
            .join("demo")
            .join("teams")
            .join("default")
            .join("team.gmap"),
        "_ = forbidden\n",
    )?;
    Ok(())
}

fn write_if_missing(path: &Path, contents: &str) -> anyhow::Result<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, contents)?;
    Ok(())
}

fn ensure_tenant_and_team(bundle: &Path, selection: &TenantSelection) -> anyhow::Result<()> {
    project::add_tenant(bundle, &selection.tenant)?;
    if let Some(team) = selection.team.as_deref()
        && !team.is_empty()
    {
        project::add_team(bundle, &selection.tenant, team)?;
    }
    Ok(())
}

fn demo_bundle_gmap_path(bundle: &Path, tenant: &str, team: Option<&str>) -> PathBuf {
    let mut path = bundle.join("tenants").join(tenant);
    if let Some(team) = team {
        path = path.join("teams").join(team).join("team.gmap");
    } else {
        path = path.join("tenant.gmap");
    }
    path
}

fn resolved_manifest_filename(tenant: &str, team: Option<&str>) -> String {
    match team {
        Some(team) => format!("{tenant}.{team}.yaml"),
        None => format!("{tenant}.yaml"),
    }
}

fn resolve_pack_refs(pack_refs: &[String], offline: bool) -> anyhow::Result<Vec<ResolvedPackInfo>> {
    use greentic_distributor_client::{
        OciPackFetcher, PackFetchOptions, oci_packs::DefaultRegistryClient,
    };

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("build tokio runtime for pack resolution")?;

    let mut opts = PackFetchOptions {
        allow_tags: true,
        offline,
        ..PackFetchOptions::default()
    };
    if let Ok(cache_dir) = std::env::var("GREENTIC_PACK_CACHE_DIR") {
        opts.cache_dir = PathBuf::from(cache_dir);
    }
    let fetcher: OciPackFetcher<DefaultRegistryClient> = OciPackFetcher::new(opts);

    let mut resolved = Vec::new();
    for reference in pack_refs {
        let mapped_ref = map_pack_reference(reference)?;
        let fetched = rt
            .block_on(fetcher.fetch_pack_to_cache(&mapped_ref))
            .with_context(|| format!("fetch pack reference {reference}"))?;
        let meta = crate::domains::read_pack_meta(&fetched.path)
            .with_context(|| format!("read pack meta from {}", fetched.path.display()))?;
        let file_name = deterministic_pack_file_name(reference, &fetched.resolved_digest);
        resolved.push(ResolvedPackInfo {
            source_ref: reference.clone(),
            mapped_ref,
            resolved_digest: fetched.resolved_digest,
            pack_id: meta.pack_id,
            entry_flows: meta.entry_flows,
            cached_path: fetched.path,
            output_path: PathBuf::from("packs").join(file_name),
        });
    }
    resolved.sort_by(|a, b| a.source_ref.cmp(&b.source_ref));
    Ok(resolved)
}

fn map_pack_reference(reference: &str) -> anyhow::Result<String> {
    let trimmed = reference.trim();
    if let Some(rest) = trimmed.strip_prefix("oci://") {
        return Ok(rest.to_string());
    }
    if let Some(rest) = trimmed.strip_prefix("repo://") {
        return map_registry_target(rest, std::env::var("GREENTIC_REPO_REGISTRY_BASE").ok())
            .ok_or_else(|| {
                anyhow!(
                    "repo:// reference {trimmed} requires GREENTIC_REPO_REGISTRY_BASE to map to OCI"
                )
            });
    }
    if let Some(rest) = trimmed.strip_prefix("store://") {
        return map_registry_target(rest, std::env::var("GREENTIC_STORE_REGISTRY_BASE").ok())
            .ok_or_else(|| {
                anyhow!(
                    "store:// reference {trimmed} requires GREENTIC_STORE_REGISTRY_BASE to map to OCI"
                )
            });
    }
    Ok(trimmed.to_string())
}

fn map_registry_target(target: &str, base: Option<String>) -> Option<String> {
    if target.contains('/') && (target.contains('@') || target.contains(':')) {
        return Some(target.to_string());
    }
    let base = base?;
    let normalized_base = base.trim_end_matches('/');
    let normalized_target = target.trim_start_matches('/');
    Some(format!("{normalized_base}/{normalized_target}"))
}

fn deterministic_pack_file_name(reference: &str, digest: &str) -> String {
    let mut slug = String::new();
    for ch in reference.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        } else {
            slug.push('-');
        }
    }
    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }
    slug = slug.trim_matches('-').to_string();
    if slug.len() > 40 {
        slug.truncate(40);
    }
    let short_digest = digest
        .trim_start_matches("sha256:")
        .chars()
        .take(12)
        .collect::<String>();
    format!("{slug}-{short_digest}.gtpack")
}

fn copy_pack_into_bundle(bundle: &Path, pack: &ResolvedPackInfo) -> anyhow::Result<()> {
    let src = pack.cached_path.clone();
    if !src.exists() {
        return Err(anyhow!("cached pack not found at {}", src.display()));
    }
    let dst = bundle.join(&pack.output_path);
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(src, dst)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_is_deterministic() {
        let req = WizardCreateRequest {
            bundle: PathBuf::from("bundle"),
            pack_refs: vec![
                "repo://zeta/pack@1".to_string(),
                "repo://alpha/pack@1".to_string(),
                "repo://alpha/pack@1".to_string(),
            ],
            tenants: vec![
                TenantSelection {
                    tenant: "demo".to_string(),
                    team: Some("default".to_string()),
                    allow_paths: vec!["pack/b".to_string(), "pack/a".to_string()],
                },
                TenantSelection {
                    tenant: "alpha".to_string(),
                    team: None,
                    allow_paths: vec!["x".to_string()],
                },
            ],
        };
        let plan = apply_create(&req, true).unwrap();
        assert_eq!(
            plan.metadata.pack_refs,
            vec![
                "repo://alpha/pack@1".to_string(),
                "repo://zeta/pack@1".to_string()
            ]
        );
        assert_eq!(plan.metadata.tenants[0].tenant, "alpha");
        assert_eq!(
            plan.metadata.tenants[1].allow_paths,
            vec!["pack/a".to_string(), "pack/b".to_string()]
        );
    }

    #[test]
    fn dry_run_does_not_create_files() {
        let temp = tempfile::tempdir().unwrap();
        let bundle = temp.path().join("demo-bundle");
        let req = WizardCreateRequest {
            bundle: bundle.clone(),
            pack_refs: Vec::new(),
            tenants: vec![TenantSelection {
                tenant: "demo".to_string(),
                team: Some("default".to_string()),
                allow_paths: vec!["packs/default".to_string()],
            }],
        };
        let _plan = apply_create(&req, true).unwrap();
        assert!(!bundle.exists());
    }

    #[test]
    fn execute_creates_bundle_and_resolved_manifest() {
        let temp = tempfile::tempdir().unwrap();
        let bundle = temp.path().join("demo-bundle");
        let req = WizardCreateRequest {
            bundle: bundle.clone(),
            pack_refs: Vec::new(),
            tenants: vec![TenantSelection {
                tenant: "demo".to_string(),
                team: Some("default".to_string()),
                allow_paths: vec!["packs/default".to_string()],
            }],
        };
        let plan = apply_create(&req, false).unwrap();
        let report = execute_create_plan(&plan, true).unwrap();
        assert!(report.bundle.exists());
        assert!(
            bundle
                .join("state")
                .join("resolved")
                .join("demo.default.yaml")
                .exists()
        );
        assert!(bundle.join("resolved").join("demo.default.yaml").exists());
    }

    #[test]
    fn update_mode_executes() {
        let temp = tempfile::tempdir().unwrap();
        let bundle = temp.path().join("demo-bundle");
        let create_req = WizardCreateRequest {
            bundle: bundle.clone(),
            pack_refs: Vec::new(),
            tenants: vec![TenantSelection {
                tenant: "demo".to_string(),
                team: None,
                allow_paths: vec!["packs/default".to_string()],
            }],
        };
        let create_plan = apply_create(&create_req, false).unwrap();
        let _ = execute_create_plan(&create_plan, true).unwrap();

        let req = WizardCreateRequest {
            bundle: bundle.clone(),
            pack_refs: Vec::new(),
            tenants: vec![TenantSelection {
                tenant: "demo".to_string(),
                team: None,
                allow_paths: vec!["packs/new".to_string()],
            }],
        };
        let plan = apply_update(&req, false).unwrap();
        assert_eq!(plan.mode, "update");
        let report = execute_update_plan(&plan, true).unwrap();
        assert!(report.bundle.exists());
    }

    #[test]
    fn remove_mode_forbids_rule() {
        let temp = tempfile::tempdir().unwrap();
        let bundle = temp.path().join("demo-bundle");
        let create_req = WizardCreateRequest {
            bundle: bundle.clone(),
            pack_refs: Vec::new(),
            tenants: vec![TenantSelection {
                tenant: "demo".to_string(),
                team: None,
                allow_paths: vec!["packs/default".to_string()],
            }],
        };
        let create_plan = apply_create(&create_req, false).unwrap();
        let _ = execute_create_plan(&create_plan, true).unwrap();

        let remove_req = WizardCreateRequest {
            bundle: bundle.clone(),
            pack_refs: Vec::new(),
            tenants: vec![TenantSelection {
                tenant: "demo".to_string(),
                team: None,
                allow_paths: vec!["packs/default".to_string()],
            }],
        };
        let remove_plan = apply_remove(&remove_req, false).unwrap();
        let _ = execute_remove_plan(&remove_plan).unwrap();
        let gmap = std::fs::read_to_string(
            bundle
                .join("tenants")
                .join("demo")
                .join("teams")
                .join("default")
                .join("team.gmap"),
        )
        .unwrap();
        assert!(gmap.contains("packs/default = forbidden"));
    }
}
