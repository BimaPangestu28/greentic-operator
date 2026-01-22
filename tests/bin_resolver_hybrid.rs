use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use greentic_operator::bin_resolver::{ResolveCtx, resolve_binary};
use greentic_operator::dev_mode::{DevProfile, DevSettingsResolved};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

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
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms).unwrap();
    }
}

#[test]
fn dev_mode_on_without_mapping_uses_path() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let bin_dir = temp.path().join("bin");
    let bin_path = bin_dir.join(binary_name("greentic-pack"));
    touch(&bin_path);

    let original_path = std::env::var("PATH").unwrap_or_default();
    let joined = std::env::join_paths([bin_dir.clone()])
        .unwrap()
        .to_string_lossy()
        .to_string();
    unsafe {
        std::env::set_var("PATH", joined);
    }

    let ctx = ResolveCtx {
        config_dir: temp.path().to_path_buf(),
        dev: Some(DevSettingsResolved {
            root: temp.path().to_path_buf(),
            profile: DevProfile::Debug,
            target_dir: None,
            repo_map: BTreeMap::new(),
        }),
        explicit_path: None,
    };
    let resolved = resolve_binary("greentic-pack", &ctx).unwrap();
    assert_eq!(resolved, bin_path);

    unsafe {
        std::env::set_var("PATH", original_path);
    }
}

#[test]
fn dev_mode_off_ignores_repo_map() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let bin_dir = temp.path().join("bin");
    let bin_path = bin_dir.join(binary_name("greentic-pack"));
    touch(&bin_path);

    let original_path = std::env::var("PATH").unwrap_or_default();
    let joined = std::env::join_paths([bin_dir.clone()])
        .unwrap()
        .to_string_lossy()
        .to_string();
    unsafe {
        std::env::set_var("PATH", joined);
    }

    let ctx = ResolveCtx {
        config_dir: temp.path().to_path_buf(),
        dev: None,
        explicit_path: None,
    };
    let resolved = resolve_binary("greentic-pack", &ctx).unwrap();
    assert_eq!(resolved, bin_path);

    unsafe {
        std::env::set_var("PATH", original_path);
    }
}
