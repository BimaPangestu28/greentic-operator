use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use crate::dev_detect::{DetectOptions, MergeSummary, detect_repo_map, merge_repo_map};
use crate::dev_mode::DevProfile;
use crate::settings::{OperatorSettings, save_settings};

#[derive(Parser)]
#[command(
    about = "Enable global dev mode.",
    long_about = "Persists dev mode settings under the operator settings file.",
    after_help = "Main options:\n  --root <PATH>\n\nOptional options:\n  --profile <debug|release> (default: debug)\n  --target-dir <PATH>\n  --no-detect\n  --detect-all"
)]
pub struct DevModeOnArgs {
    #[arg(long)]
    root: PathBuf,
    #[arg(long, value_enum, default_value_t = ProfileArg::Debug)]
    profile: ProfileArg,
    #[arg(long)]
    target_dir: Option<PathBuf>,
    #[arg(long)]
    no_detect: bool,
    #[arg(long)]
    detect_all: bool,
}

#[derive(Parser)]
#[command(
    about = "Disable global dev mode.",
    long_about = "Persists dev mode disabled under the operator settings file.",
    after_help = "Main options:\n  (none)\n\nOptional options:\n  (none)"
)]
pub struct DevModeOffArgs {}

#[derive(Parser)]
#[command(
    about = "Show global dev mode settings.",
    long_about = "Prints the persisted dev mode configuration.",
    after_help = "Main options:\n  (none)\n\nOptional options:\n  (none)"
)]
pub struct DevModeStatusArgs {}

#[derive(Parser)]
#[command(
    about = "Detect repo map entries from a workspace root.",
    long_about = "Scans immediate child repos for built binaries and optionally persists repo_map entries.",
    after_help = "Main options:\n  --root <PATH>\n\nOptional options:\n  --profile <debug|release> (default: debug)\n  --dry-run\n  --detect-all"
)]
pub struct DevModeDetectArgs {
    #[arg(long)]
    root: PathBuf,
    #[arg(long, value_enum, default_value_t = ProfileArg::Debug)]
    profile: ProfileArg,
    #[arg(long)]
    dry_run: bool,
    #[arg(long)]
    detect_all: bool,
}

#[derive(Parser)]
pub struct DevModeMapCommand {
    #[command(subcommand)]
    command: DevModeMapSubcommand,
}

#[derive(Subcommand)]
pub enum DevModeMapSubcommand {
    Set(DevModeMapSetArgs),
    Rm(DevModeMapRmArgs),
    Ls(DevModeMapListArgs),
}

#[derive(Parser)]
#[command(
    about = "Set a repo map entry.",
    long_about = "Maps a binary name to a repo directory name for dev resolution.",
    after_help = "Main options:\n  <BINARY>\n  <REPO>\n\nOptional options:\n  (none)"
)]
pub struct DevModeMapSetArgs {
    binary: String,
    repo: String,
}

#[derive(Parser)]
#[command(
    about = "Remove a repo map entry.",
    long_about = "Deletes a binary-to-repo mapping from the dev settings.",
    after_help = "Main options:\n  <BINARY>\n\nOptional options:\n  (none)"
)]
pub struct DevModeMapRmArgs {
    binary: String,
}

#[derive(Parser)]
#[command(
    about = "List repo map entries.",
    long_about = "Prints the current binary-to-repo mappings.",
    after_help = "Main options:\n  (none)\n\nOptional options:\n  (none)"
)]
pub struct DevModeMapListArgs {}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ProfileArg {
    Debug,
    Release,
}

impl DevModeOnArgs {
    pub fn run(self, mut settings: OperatorSettings) -> anyhow::Result<OperatorSettings> {
        if !self.root.exists() {
            return Err(anyhow::anyhow!(
                "dev root does not exist: {}",
                self.root.display()
            ));
        }
        settings.dev.enabled = true;
        settings.dev.root = Some(self.root.clone());
        settings.dev.profile = self.profile.into();
        settings.dev.target_dir = self.target_dir;
        if !self.no_detect {
            let profile = self.profile.into();
            let (result, used_profile) = detect_with_fallback(&self.root, profile)?;
            let summary = merge_repo_map(&mut settings.dev.repo_map, &result, self.detect_all);
            print_detect_summary(&result, &summary, used_profile);
        }
        save_settings(&settings)?;
        println!("dev mode: enabled");
        Ok(settings)
    }
}

impl DevModeOffArgs {
    pub fn run(self, mut settings: OperatorSettings) -> anyhow::Result<OperatorSettings> {
        settings.dev.enabled = false;
        save_settings(&settings)?;
        println!("dev mode: disabled");
        Ok(settings)
    }
}

impl DevModeStatusArgs {
    pub fn run(self, settings: OperatorSettings) -> anyhow::Result<OperatorSettings> {
        println!("enabled: {}", settings.dev.enabled);
        println!(
            "root: {}",
            settings
                .dev
                .root
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "unset".to_string())
        );
        println!(
            "profile: {}",
            match settings.dev.profile {
                DevProfile::Debug => "debug",
                DevProfile::Release => "release",
            }
        );
        println!(
            "target_dir: {}",
            settings
                .dev
                .target_dir
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "unset".to_string())
        );
        println!("repo_map entries: {}", settings.dev.repo_map.len());
        if settings.dev.repo_map.is_empty() {
            return Ok(settings);
        }

        println!("repo_map:");
        for (binary, repo) in &settings.dev.repo_map {
            if let Some(path) = resolve_repo_binary(&settings, repo, binary) {
                let status = if path.exists() { "ok" } else { "missing" };
                println!("  {binary} -> {repo} ({status} {})", path.display());
            } else {
                println!("  {binary} -> {repo} (root unset)");
            }
        }
        Ok(settings)
    }
}

impl DevModeDetectArgs {
    pub fn run(self, mut settings: OperatorSettings) -> anyhow::Result<OperatorSettings> {
        if !self.root.exists() {
            return Err(anyhow::anyhow!(
                "dev root does not exist: {}",
                self.root.display()
            ));
        }
        let profile = self.profile.into();
        let (result, used_profile) = detect_with_fallback(&self.root, profile)?;
        let mut repo_map = settings.dev.repo_map.clone();
        let summary = merge_repo_map(&mut repo_map, &result, self.detect_all);
        print_detect_summary(&result, &summary, used_profile);
        if !self.dry_run {
            settings.dev.repo_map = repo_map;
            save_settings(&settings)?;
        }
        Ok(settings)
    }
}

impl DevModeMapCommand {
    pub fn run(self, settings: OperatorSettings) -> anyhow::Result<OperatorSettings> {
        match self.command {
            DevModeMapSubcommand::Set(args) => args.run(settings),
            DevModeMapSubcommand::Rm(args) => args.run(settings),
            DevModeMapSubcommand::Ls(args) => args.run(settings),
        }
    }
}

impl DevModeMapSetArgs {
    pub fn run(self, mut settings: OperatorSettings) -> anyhow::Result<OperatorSettings> {
        settings.dev.repo_map.insert(self.binary, self.repo);
        save_settings(&settings)?;
        Ok(settings)
    }
}

impl DevModeMapRmArgs {
    pub fn run(self, mut settings: OperatorSettings) -> anyhow::Result<OperatorSettings> {
        settings.dev.repo_map.remove(&self.binary);
        save_settings(&settings)?;
        Ok(settings)
    }
}

impl DevModeMapListArgs {
    pub fn run(self, settings: OperatorSettings) -> anyhow::Result<OperatorSettings> {
        if settings.dev.repo_map.is_empty() {
            println!("repo_map: (empty)");
        } else {
            for (binary, repo) in &settings.dev.repo_map {
                println!("{binary} -> {repo}");
            }
        }
        Ok(settings)
    }
}

impl From<ProfileArg> for DevProfile {
    fn from(value: ProfileArg) -> Self {
        match value {
            ProfileArg::Debug => DevProfile::Debug,
            ProfileArg::Release => DevProfile::Release,
        }
    }
}

fn detect_with_fallback(
    root: &std::path::Path,
    profile: DevProfile,
) -> anyhow::Result<(crate::dev_detect::DetectResult, DevProfile)> {
    let result = detect_repo_map(&DetectOptions {
        root: root.to_path_buf(),
        profile,
    })?;
    if !result.found.is_empty() {
        return Ok((result, profile));
    }
    let fallback = match profile {
        DevProfile::Debug => DevProfile::Release,
        DevProfile::Release => DevProfile::Debug,
    };
    let fallback_result = detect_repo_map(&DetectOptions {
        root: root.to_path_buf(),
        profile: fallback,
    })?;
    Ok((fallback_result, fallback))
}

fn print_detect_summary(
    result: &crate::dev_detect::DetectResult,
    summary: &MergeSummary,
    profile: DevProfile,
) {
    let mut repos = std::collections::BTreeSet::new();
    for found in &result.found {
        repos.insert(found.repo.as_str());
    }
    println!(
        "detected {} binaries across {} repos (profile={:?})",
        result.found.len(),
        repos.len(),
        profile
    );
    println!(
        "added: {}, skipped mapped: {}",
        summary.added, summary.skipped_mapped
    );
    if !summary.skipped_mapped_bins.is_empty() {
        println!("skipped mapped:");
        for bin in &summary.skipped_mapped_bins {
            println!("  {bin}");
        }
    }
    if !summary.ambiguous.is_empty() {
        println!("ambiguous:");
        for (bin, repos) in &summary.ambiguous {
            println!("  {bin} -> {:?}", repos);
        }
    }
}

fn resolve_repo_binary(
    settings: &OperatorSettings,
    repo: &str,
    binary: &str,
) -> Option<std::path::PathBuf> {
    let root = settings.dev.root.as_ref()?;
    let target = settings
        .dev
        .target_dir
        .as_ref()
        .cloned()
        .unwrap_or_else(|| root.join(repo).join("target"));
    let name = if cfg!(windows) {
        if binary.ends_with(".exe") {
            binary.to_string()
        } else {
            format!("{binary}.exe")
        }
    } else {
        binary.to_string()
    };
    Some(
        target
            .join(crate::dev_mode::profile_dir(settings.dev.profile))
            .join(name),
    )
}
