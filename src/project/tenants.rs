use std::path::Path;

pub fn add_tenant(root: &Path, tenant: &str) -> anyhow::Result<()> {
    let tenant_dir = root.join("tenants").join(tenant);
    std::fs::create_dir_all(tenant_dir.join("teams"))?;
    let gmap_path = tenant_dir.join("tenant.gmap");
    if !gmap_path.exists() {
        std::fs::write(gmap_path, "_ = forbidden\n")?;
    }
    Ok(())
}

pub fn remove_tenant(root: &Path, tenant: &str) -> anyhow::Result<()> {
    let tenant_dir = root.join("tenants").join(tenant);
    if tenant_dir.exists() {
        std::fs::remove_dir_all(tenant_dir)?;
    }
    Ok(())
}

pub fn list_tenants(root: &Path) -> anyhow::Result<Vec<String>> {
    let tenants_dir = root.join("tenants");
    let mut tenants = Vec::new();
    if !tenants_dir.exists() {
        return Ok(tenants);
    }
    for entry in std::fs::read_dir(tenants_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            tenants.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    tenants.sort();
    Ok(tenants)
}

pub fn add_team(root: &Path, tenant: &str, team: &str) -> anyhow::Result<()> {
    let team_dir = root.join("tenants").join(tenant).join("teams").join(team);
    std::fs::create_dir_all(&team_dir)?;
    let gmap_path = team_dir.join("team.gmap");
    if !gmap_path.exists() {
        std::fs::write(gmap_path, "_ = forbidden\n")?;
    }
    Ok(())
}

pub fn remove_team(root: &Path, tenant: &str, team: &str) -> anyhow::Result<()> {
    let team_dir = root.join("tenants").join(tenant).join("teams").join(team);
    if team_dir.exists() {
        std::fs::remove_dir_all(team_dir)?;
    }
    Ok(())
}

pub fn list_teams(root: &Path, tenant: &str) -> anyhow::Result<Vec<String>> {
    let teams_dir = root.join("tenants").join(tenant).join("teams");
    let mut teams = Vec::new();
    if !teams_dir.exists() {
        return Ok(teams);
    }
    for entry in std::fs::read_dir(teams_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            teams.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    teams.sort();
    Ok(teams)
}
