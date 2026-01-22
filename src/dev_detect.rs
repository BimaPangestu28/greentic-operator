use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::dev_mode::{DevProfile, profile_dir};

pub struct DetectOptions {
    pub root: PathBuf,
    pub profile: DevProfile,
}

pub struct FoundBinary {
    pub bin: String,
    pub repo: String,
    pub path: PathBuf,
}

pub struct DetectResult {
    pub found: Vec<FoundBinary>,
    pub unambiguous: BTreeMap<String, String>,
    pub ambiguous: BTreeMap<String, Vec<String>>,
}

pub struct MergeSummary {
    pub added: usize,
    pub skipped_mapped: usize,
    pub ambiguous: BTreeMap<String, Vec<String>>,
    pub skipped_mapped_bins: Vec<String>,
}

pub fn detect_repo_map(options: &DetectOptions) -> anyhow::Result<DetectResult> {
    let mut found = Vec::new();
    let mut candidates: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for entry in std::fs::read_dir(&options.root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let repo_name = entry.file_name().to_string_lossy().to_string();
        let repo_path = entry.path();
        let target = repo_path.join("target").join(profile_dir(options.profile));
        collect_binaries(&repo_name, &target, &mut found, &mut candidates)?;
    }

    let mut unambiguous = BTreeMap::new();
    let mut ambiguous = BTreeMap::new();

    for (bin, repos) in candidates {
        if repos.len() == 1 {
            unambiguous.insert(bin, repos.into_iter().next().unwrap());
        } else if repos.len() > 1 {
            ambiguous.insert(bin, repos.into_iter().collect());
        }
    }

    Ok(DetectResult {
        found,
        unambiguous,
        ambiguous,
    })
}

pub fn merge_repo_map(
    existing: &mut BTreeMap<String, String>,
    result: &DetectResult,
    _detect_all: bool,
) -> MergeSummary {
    let mut summary = MergeSummary {
        added: 0,
        skipped_mapped: 0,
        ambiguous: result.ambiguous.clone(),
        skipped_mapped_bins: Vec::new(),
    };

    for (bin, repo) in &result.unambiguous {
        if existing.contains_key(bin) {
            summary.skipped_mapped += 1;
            summary.skipped_mapped_bins.push(bin.clone());
            continue;
        }
        existing.insert(bin.clone(), repo.clone());
        summary.added += 1;
    }

    summary
}

pub fn is_on_path(bin: &str) -> bool {
    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    for dir in std::env::split_paths(&path_var) {
        if path_has_binary(&dir, bin) {
            return true;
        }
    }
    false
}

fn collect_binaries(
    repo_name: &str,
    target_dir: &Path,
    found: &mut Vec<FoundBinary>,
    candidates: &mut BTreeMap<String, BTreeSet<String>>,
) -> anyhow::Result<()> {
    if !target_dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(target_dir)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(bin) = detect_bin_name(&path)? {
            found.push(FoundBinary {
                bin: bin.clone(),
                repo: repo_name.to_string(),
                path: path.clone(),
            });
            candidates
                .entry(bin)
                .or_default()
                .insert(repo_name.to_string());
        }
    }
    Ok(())
}

fn detect_bin_name(path: &Path) -> anyhow::Result<Option<String>> {
    let file_name = match path.file_name().and_then(|name| name.to_str()) {
        Some(value) => value,
        None => return Ok(None),
    };
    if has_library_extension(file_name) {
        return Ok(None);
    }
    if cfg!(windows) {
        if !file_name.ends_with(".exe") {
            return Ok(None);
        }
        let bin = file_name.trim_end_matches(".exe").to_string();
        return Ok(Some(bin));
    }

    let metadata = std::fs::metadata(path)?;
    if !metadata.is_file() {
        return Ok(None);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Ok(None);
        }
    }
    Ok(Some(file_name.to_string()))
}

fn has_library_extension(file_name: &str) -> bool {
    let Some(ext) = Path::new(file_name).extension().and_then(|e| e.to_str()) else {
        return false;
    };
    matches!(ext, "so" | "dylib" | "dll" | "rlib" | "a")
}

fn path_has_binary(dir: &Path, bin: &str) -> bool {
    if cfg!(windows) {
        return dir.join(format!("{bin}.exe")).is_file();
    }
    dir.join(bin).is_file()
}
