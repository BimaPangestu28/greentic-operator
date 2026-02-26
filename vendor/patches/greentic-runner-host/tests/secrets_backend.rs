use std::env;

use greentic_runner_host::secrets::SecretsBackend;
use serial_test::serial;

#[test]
#[serial]
fn env_backend_rejected_in_prod() {
    let prev = env::var("GREENTIC_ENV").ok();
    unsafe {
        env::set_var("GREENTIC_ENV", "prod");
    }

    let result = SecretsBackend::Env.build_manager();
    assert!(result.is_err());

    match prev {
        Some(value) => unsafe {
            env::set_var("GREENTIC_ENV", value);
        },
        None => unsafe {
            env::remove_var("GREENTIC_ENV");
        },
    }
}

#[test]
#[serial]
fn env_backend_allowed_in_local() {
    let prev = env::var("GREENTIC_ENV").ok();
    unsafe {
        env::set_var("GREENTIC_ENV", "local");
    }

    let result = SecretsBackend::Env.build_manager();
    assert!(result.is_ok());

    match prev {
        Some(value) => unsafe {
            env::set_var("GREENTIC_ENV", value);
        },
        None => unsafe {
            env::remove_var("GREENTIC_ENV");
        },
    }
}
