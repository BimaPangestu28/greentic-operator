use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use directories_next::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::dev_mode::{DevMode, DevProfile, DevSettings};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct OperatorSettings {
    #[serde(default)]
    pub dev: DevSettingsGlobal,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct DevSettingsGlobal {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub root: Option<PathBuf>,
    #[serde(default)]
    pub profile: DevProfile,
    #[serde(default)]
    pub target_dir: Option<PathBuf>,
    #[serde(default)]
    pub repo_map: BTreeMap<String, String>,
}

pub fn load_settings() -> anyhow::Result<OperatorSettings> {
    let path = settings_path()?;
    if !path.exists() {
        return Ok(OperatorSettings::default());
    }
    let contents = std::fs::read_to_string(&path)?;
    let settings: OperatorSettings = serde_yaml_bw::from_str(&contents)?;
    Ok(settings)
}

pub fn save_settings(settings: &OperatorSettings) -> anyhow::Result<()> {
    let path = settings_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let contents = serde_yaml_bw::to_string(settings)?;
    std::fs::write(&path, contents)?;
    Ok(())
}

pub fn settings_path() -> anyhow::Result<PathBuf> {
    if let Ok(value) = std::env::var("GREENTIC_OPERATOR_CONFIG_DIR") {
        return Ok(Path::new(&value).join("settings.yaml"));
    }
    let dirs = ProjectDirs::from("", "greentic", "operator")
        .ok_or_else(|| anyhow::anyhow!("unable to determine config directory"))?;
    Ok(dirs.config_dir().join("settings.yaml"))
}

impl DevSettingsGlobal {
    pub fn to_dev_settings(&self) -> DevSettings {
        DevSettings {
            mode: Some(if self.enabled {
                DevMode::On
            } else {
                DevMode::Off
            }),
            root: self.root.clone(),
            profile: Some(self.profile),
            target_dir: self.target_dir.clone(),
            repo_map: self.repo_map.clone(),
        }
    }
}
