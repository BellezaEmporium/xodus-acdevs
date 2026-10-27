use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) fn verify_wine(wine: &str) -> Result<String, String> {
    if wine.trim().is_empty() {
        return Err("Enter the path or command name for Wine.".to_owned());
    }
    let output = Command::new(wine)
        .arg("--version")
        .output()
        .map_err(|error| format!("Could not run Wine at '{wine}': {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Wine check failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let version = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    Ok(if version.is_empty() {
        "Wine is available.".to_owned()
    } else {
        format!("Wine is available: {version}")
    })
}

pub(crate) fn initialize_wine_prefix(wine: &str, prefix: &str) -> Result<String, String> {
    verify_wine(wine)?;
    if prefix.trim().is_empty() {
        return Err("Enter a path for the Wine prefix.".to_owned());
    }

    let prefix_path = PathBuf::from(prefix);
    std::fs::create_dir_all(&prefix_path)
        .map_err(|error| format!("Could not create Wine prefix folder: {error}"))?;
    let wineboot = Path::new(wine)
        .parent()
        .map(|parent| {
            parent.join(if cfg!(windows) {
                "wineboot.exe"
            } else {
                "wineboot"
            })
        })
        .filter(|path| path.is_file())
        .unwrap_or_else(|| {
            PathBuf::from(if cfg!(windows) {
                "wineboot.exe"
            } else {
                "wineboot"
            })
        });
    let output = Command::new(&wineboot)
        .arg("--init")
        .env("WINEPREFIX", &prefix_path)
        .output()
        .map_err(|error| format!("Could not start wineboot: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Wine prefix initialization failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    if !prefix_path.join("system.reg").is_file() {
        return Err(format!(
            "wineboot completed but {} does not look like a Wine prefix.",
            prefix_path.display()
        ));
    }
    Ok(format!(
        "Wine prefix initialized at {}.",
        prefix_path.display()
    ))
}
