use std::collections::BTreeMap;
use std::path::Path;

use greentic_operator::bin_resolver::{ResolveCtx, resolve_binary};
use greentic_operator::dev_mode::{DevProfile, DevSettingsResolved};

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
fn resolves_explicit_path_first() {
    let temp = tempfile::tempdir().unwrap();
    let explicit = temp.path().join("bin").join(binary_name("greentic-pack"));
    touch(&explicit);

    let ctx = ResolveCtx {
        config_dir: temp.path().to_path_buf(),
        dev: Some(DevSettingsResolved {
            root: temp.path().join("missing-root"),
            profile: DevProfile::Debug,
            target_dir: None,
            repo_map: BTreeMap::new(),
        }),
        explicit_path: Some(explicit.clone()),
    };
    let resolved = resolve_binary("greentic-pack", &ctx).unwrap();
    assert_eq!(resolved, explicit);
}

#[test]
fn resolves_from_dev_root() {
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("greentic-pack");
    let bin_path = repo
        .join("target")
        .join("debug")
        .join(binary_name("greentic-pack"));
    touch(&bin_path);

    let ctx = ResolveCtx {
        config_dir: temp.path().to_path_buf(),
        dev: Some(DevSettingsResolved {
            root: temp.path().to_path_buf(),
            profile: DevProfile::Debug,
            target_dir: None,
            repo_map: BTreeMap::from([("greentic-pack".to_string(), "greentic-pack".to_string())]),
        }),
        explicit_path: None,
    };
    let resolved = resolve_binary("greentic-pack", &ctx).unwrap();
    assert_eq!(resolved, bin_path);
}

#[test]
fn resolves_from_local_bin_dir() {
    let temp = tempfile::tempdir().unwrap();
    let bin_path = temp.path().join("bin").join(binary_name("greentic-pack"));
    touch(&bin_path);

    let ctx = ResolveCtx {
        config_dir: temp.path().to_path_buf(),
        dev: None,
        explicit_path: None,
    };
    let resolved = resolve_binary("greentic-pack", &ctx).unwrap();
    assert_eq!(resolved, bin_path);
}
