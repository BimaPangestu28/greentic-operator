use std::fs;
use std::path::Path;

use serde_yaml_bw::Value;

#[test]
fn resolve_manifests_for_tenants_and_teams() {
    let temp = tempfile::tempdir().unwrap();
    let project_root = temp.path().join("project");

    fs::create_dir_all(project_root.join("providers").join("messaging")).unwrap();
    fs::create_dir_all(project_root.join("packs").join("pack1")).unwrap();
    fs::write(
        project_root
            .join("providers")
            .join("messaging")
            .join("provider.gtpack"),
        "pack",
    )
    .unwrap();
    fs::write(project_root.join("packs").join("pack2.gtpack"), "pack").unwrap();

    fs::create_dir_all(
        project_root
            .join("tenants")
            .join("alpha")
            .join("teams")
            .join("team1"),
    )
    .unwrap();
    fs::write(
        project_root
            .join("tenants")
            .join("alpha")
            .join("tenant.gmap"),
        "_ = forbidden\n",
    )
    .unwrap();
    fs::write(
        project_root
            .join("tenants")
            .join("alpha")
            .join("teams")
            .join("team1")
            .join("team.gmap"),
        "_ = forbidden\n",
    )
    .unwrap();

    fs::create_dir_all(project_root.join("tenants").join("beta")).unwrap();
    fs::write(
        project_root
            .join("tenants")
            .join("beta")
            .join("tenant.gmap"),
        "_ = forbidden\n",
    )
    .unwrap();

    let old_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp.path()).unwrap();
    greentic_operator::project::sync_project(Path::new("project")).unwrap();
    std::env::set_current_dir(old_dir).unwrap();

    let team_manifest = fs::read_to_string(
        project_root
            .join("state")
            .join("resolved")
            .join("alpha.team1.yaml"),
    )
    .unwrap();
    let team_value: Value = serde_yaml_bw::from_str(&team_manifest).unwrap();
    assert_eq!(
        team_value.get("project_root").unwrap().as_str().unwrap(),
        "project"
    );
    assert_eq!(team_value.get("tenant").unwrap().as_str().unwrap(), "alpha");
    assert_eq!(team_value.get("team").unwrap().as_str().unwrap(), "team1");
    let providers = team_value.get("providers").unwrap();
    let messaging = providers.get("messaging").unwrap().as_sequence().unwrap();
    assert!(
        messaging
            .iter()
            .any(|value| value.as_str() == Some("providers/messaging/provider.gtpack"))
    );

    let tenant_manifest = fs::read_to_string(
        project_root
            .join("state")
            .join("resolved")
            .join("beta.yaml"),
    )
    .unwrap();
    let tenant_value: Value = serde_yaml_bw::from_str(&tenant_manifest).unwrap();
    assert!(tenant_value.get("team").is_none());
    let packs = tenant_value.get("packs").unwrap().as_sequence().unwrap();
    assert!(
        packs
            .iter()
            .any(|value| value.as_str() == Some("packs/pack1"))
    );
    assert!(
        packs
            .iter()
            .any(|value| value.as_str() == Some("packs/pack2.gtpack"))
    );
}
