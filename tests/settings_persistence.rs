use std::sync::{Mutex, OnceLock};

use greentic_operator::settings::{OperatorSettings, load_settings, save_settings};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[test]
fn load_settings_defaults_when_missing() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let temp = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("GREENTIC_OPERATOR_CONFIG_DIR", temp.path());
    }
    let settings = load_settings().unwrap();
    assert!(!settings.dev.enabled);
    unsafe {
        std::env::remove_var("GREENTIC_OPERATOR_CONFIG_DIR");
    }
}

#[test]
fn save_and_load_settings_roundtrip() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let temp = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("GREENTIC_OPERATOR_CONFIG_DIR", temp.path());
    }
    let mut settings = OperatorSettings::default();
    settings.dev.enabled = true;
    settings.dev.root = Some(temp.path().join("workspace"));
    save_settings(&settings).unwrap();

    let loaded = load_settings().unwrap();
    assert!(loaded.dev.enabled);
    assert_eq!(loaded.dev.root, settings.dev.root);
    unsafe {
        std::env::remove_var("GREENTIC_OPERATOR_CONFIG_DIR");
    }
}
