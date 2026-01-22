use std::path::Path;

pub fn demo_doctor(bundle_root: &Path, pack_command: &Path) -> anyhow::Result<()> {
    let packs_root = bundle_root.join("packs");
    if !packs_root.exists() {
        return Err(anyhow::anyhow!("Bundle packs directory not found."));
    }

    let mut packs = Vec::new();
    collect_gtpacks(&packs_root, &mut packs)?;
    if packs.is_empty() {
        return Err(anyhow::anyhow!("No .gtpack files found in bundle."));
    }

    for pack in packs {
        let status = std::process::Command::new(pack_command)
            .args(["doctor", pack.to_str().unwrap_or_default()])
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!(
                "greentic-pack doctor failed for {}",
                pack.display()
            ));
        }
    }

    Ok(())
}

fn collect_gtpacks(dir: &Path, packs: &mut Vec<std::path::PathBuf>) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect_gtpacks(&path, packs)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("gtpack") {
            packs.push(path);
        }
    }
    Ok(())
}
