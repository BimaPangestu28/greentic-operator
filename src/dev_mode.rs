use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DevMode {
    Auto,
    On,
    Off,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Default, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DevProfile {
    #[default]
    Debug,
    Release,
}

#[derive(Clone, Debug, Deserialize, Default)]
pub struct DevSettings {
    #[serde(default)]
    pub mode: Option<DevMode>,
    #[serde(default)]
    pub root: Option<PathBuf>,
    #[serde(default)]
    pub profile: Option<DevProfile>,
    #[serde(default)]
    pub target_dir: Option<PathBuf>,
    #[serde(default)]
    pub repo_map: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct DevCliOverrides {
    pub mode: Option<DevMode>,
    pub root: Option<PathBuf>,
    pub profile: Option<DevProfile>,
    pub target_dir: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct DevSettingsResolved {
    pub root: PathBuf,
    pub profile: DevProfile,
    pub target_dir: Option<PathBuf>,
    pub repo_map: BTreeMap<String, String>,
}

pub fn effective_dev_settings(
    cli: DevCliOverrides,
    config: Option<DevSettings>,
    config_dir: &Path,
) -> anyhow::Result<Option<DevSettingsResolved>> {
    let config = config.unwrap_or_default();
    let mode = cli.mode.or(config.mode).unwrap_or(DevMode::Auto);
    let root = cli.root.or(config.root);
    let profile = cli.profile.or(config.profile).unwrap_or(DevProfile::Debug);
    let target_dir = cli.target_dir.or(config.target_dir);
    let repo_map = config.repo_map;

    match mode {
        DevMode::Off => Ok(None),
        DevMode::Auto => {
            let Some(root) = root else {
                return Ok(None);
            };
            let root = resolve_relative(config_dir, root);
            Ok(Some(DevSettingsResolved {
                root,
                profile,
                target_dir: target_dir.map(|path| resolve_relative(config_dir, path)),
                repo_map,
            }))
        }
        DevMode::On => {
            let Some(root) = root else {
                return Err(anyhow::anyhow!(
                    "dev mode is on but no dev root was provided"
                ));
            };
            let root = resolve_relative(config_dir, root);
            Ok(Some(DevSettingsResolved {
                root,
                profile,
                target_dir: target_dir.map(|path| resolve_relative(config_dir, path)),
                repo_map,
            }))
        }
    }
}

pub fn profile_dir(profile: DevProfile) -> &'static str {
    match profile {
        DevProfile::Debug => "debug",
        DevProfile::Release => "release",
    }
}

pub fn merge_settings(
    primary: Option<DevSettings>,
    fallback: Option<DevSettings>,
) -> Option<DevSettings> {
    match (primary, fallback) {
        (None, None) => None,
        (Some(primary), None) => Some(primary),
        (None, Some(fallback)) => Some(fallback),
        (Some(primary), Some(fallback)) => Some(DevSettings {
            mode: primary.mode.or(fallback.mode),
            root: primary.root.or(fallback.root),
            profile: primary.profile.or(fallback.profile),
            target_dir: primary.target_dir.or(fallback.target_dir),
            repo_map: if primary.repo_map.is_empty() {
                fallback.repo_map
            } else {
                primary.repo_map
            },
        }),
    }
}

fn resolve_relative(base: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        base.join(path)
    }
}
