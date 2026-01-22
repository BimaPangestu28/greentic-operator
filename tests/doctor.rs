use std::path::Path;

use greentic_operator::doctor;
use greentic_operator::domains::{self, Domain};

#[test]
fn validator_lookup_prefers_local_path() {
    let temp = tempfile::tempdir().unwrap();
    let validators = temp
        .path()
        .join("validators")
        .join("messaging")
        .join("validators-messaging.gtpack");
    std::fs::create_dir_all(validators.parent().unwrap()).unwrap();
    std::fs::write(&validators, "stub").unwrap();

    let found = domains::validator_pack_path(temp.path(), Domain::Messaging);
    assert_eq!(found, Some(validators));
}

#[test]
fn doctor_args_include_validator_packs() {
    let pack = Path::new("providers/messaging/sample.gtpack");
    let args = doctor::build_doctor_args(
        pack,
        &[Path::new("validators/messaging/validators-messaging.gtpack").to_path_buf()],
        true,
    );
    assert_eq!(
        args,
        vec![
            "doctor",
            "providers/messaging/sample.gtpack",
            "--strict",
            "--validator-pack",
            "validators/messaging/validators-messaging.gtpack",
        ]
    );
}
