use std::collections::BTreeMap;
use std::path::Path;

use greentic_operator::bin_resolver::{ResolveCtx, resolve_binary};
use greentic_operator::dev_mode::{
    DevCliOverrides, DevProfile, effective_dev_settings, merge_settings,
};
use greentic_operator::settings::DevSettingsGlobal;

fn binary_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn touch(path: &Path) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, "stub").unwrap();
}

#[test]
fn global_dev_settings_enable_resolution() {
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("greentic-pack");
    let bin_path = repo
        .join("target")
        .join("debug")
        .join(binary_name("greentic-pack"));
    touch(&bin_path);

    let global = DevSettingsGlobal {
        enabled: true,
        root: Some(temp.path().to_path_buf()),
        profile: DevProfile::Debug,
        target_dir: None,
        repo_map: BTreeMap::from([("greentic-pack".to_string(), "greentic-pack".to_string())]),
    };
    let merged = merge_settings(None, Some(global.to_dev_settings()));
    let dev_settings = effective_dev_settings(
        DevCliOverrides {
            mode: None,
            root: None,
            profile: None,
            target_dir: None,
        },
        merged,
        temp.path(),
    )
    .unwrap()
    .unwrap();

    let resolved = resolve_binary(
        "greentic-pack",
        &ResolveCtx {
            config_dir: temp.path().to_path_buf(),
            dev: Some(dev_settings),
            explicit_path: None,
        },
    )
    .unwrap();
    assert_eq!(resolved, bin_path);
}

#[test]
fn global_dev_settings_disabled_returns_none() {
    let global = DevSettingsGlobal::default();
    let merged = merge_settings(None, Some(global.to_dev_settings()));
    let dev_settings = effective_dev_settings(
        DevCliOverrides {
            mode: None,
            root: None,
            profile: None,
            target_dir: None,
        },
        merged,
        Path::new("."),
    )
    .unwrap();
    assert!(dev_settings.is_none());
}
