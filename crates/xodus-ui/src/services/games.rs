use crate::models::game::{Game, GameProvider};
use std::path::{Path, PathBuf};
use crate::services::cli::cli_command;
use crate::services::wine::verify_wine;

pub fn discover_games(root: &Path) -> std::io::Result<Vec<Game>> {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Vec::new());
        }
        Err(error) => return Err(error),
    };
    let mut games = Vec::new();

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let manifest_path = path.join(".xodus-game");
        let package_marker = path.join(".xodus-streaming.msixvc");
        let manifest = match std::fs::read_to_string(&manifest_path) {
            Ok(manifest) => Some(manifest),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };
        if !package_marker.is_file() && manifest.is_none() {
            continue;
        }

        let mut title = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let mut executable = None;
        let mut provider = GameProvider::Xodus;
        if let Some(manifest) = manifest {
            for line in manifest.lines() {
                let Some((key, value)) = line.split_once('=') else {
                    continue;
                };
                match key.trim() {
                    "title" if !value.trim().is_empty() => title = value.trim().to_owned(),
                    "provider" => provider = GameProvider::from_manifest(value),
                    "executable" if !value.trim().is_empty() => {
                        executable = Some(value.trim().to_owned());
                    }
                    _ => {}
                }
            }
        }
        games.push(Game {
            path,
            title,
            executable,
            provider,
        });
    }

    games.sort_by(|left, right| left.title.to_lowercase().cmp(&right.title.to_lowercase()));
    Ok(games)
}

pub fn launch_game(
    game_path: &Path,
    executable: Option<&str>,
    wine: &str,
    prefix: &str,
) -> Result<String, String> {
    verify_wine(wine)?;
    if prefix.trim().is_empty() {
        return Err("Set a Wine prefix in Settings before running this game.".to_owned());
    }
    if !Path::new(prefix).join("system.reg").is_file() {
        return Err("The selected Wine prefix is not initialized. Initialize it in Settings first.".to_owned());
    }
    #[cfg(not(unix))]
    {
        let _ = (game_path, executable, wine, prefix);
        return Err("Launching Xodus games through Wine is only supported on Unix.".to_owned());
    }
    #[cfg(unix)]
    {
        let mut command = cli_command();
        command.arg("run").arg(game_path).arg(wine);
        if let Some(executable) = executable {
            command.arg("--exe").arg(executable);
        }
        let child = command
            .env("WINEPREFIX", prefix)
            .spawn()
            .map_err(|error| format!("Could not start xodus-cli run: {error}"))?;
        drop(child);
        Ok(format!("Started {} through xodus-cli.", game_path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_installed_games_and_provider_metadata() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "xodus-ui-library-test-{}-{unique}",
            std::process::id()
        ));
        let xbox_game = root.join("Reincarnation");
        let steam_game = root.join("Example Steam Game");
        let unrelated = root.join("Not a game");
        std::fs::create_dir_all(&xbox_game).expect("create Xodus game folder");
        std::fs::create_dir_all(&steam_game).expect("create provider game folder");
        std::fs::create_dir_all(&unrelated).expect("create unrelated folder");
        std::fs::write(xbox_game.join(".xodus-streaming.msixvc"), b"").expect("create marker");
        std::fs::write(
            steam_game.join(".xodus-game"),
            "title=Example Steam Game\nprovider=steam\nexecutable=game.exe\n",
        )
        .expect("create provider manifest");

        let games = discover_games(&root).expect("discover games");
        assert_eq!(games.len(), 2);
        let steam = games
            .iter()
            .find(|game| game.title == "Example Steam Game")
            .expect("find provider game");
        assert_eq!(steam.executable.as_deref(), Some("game.exe"));
        assert_eq!(
            steam.provider.ipc_name().as_deref(),
            Some("steam - Steam")
        );
        assert!(games.iter().any(|game| {
            game.title == "Reincarnation" && game.provider == GameProvider::Xodus
        }));

        std::fs::remove_dir_all(root).expect("clean up test library");
    }
}