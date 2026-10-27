use std::path::PathBuf;

pub(crate) struct Settings {
    pub games_directory: String,
    pub wine_path: String,
    pub wine_prefix: String,
    pub last_played: Option<PathBuf>,
}

impl Settings {
    pub(crate) fn defaults() -> Self {
        let data_dir = data_directory();
        Self {
            games_directory: data_dir.join("games").to_string_lossy().into_owned(),
            wine_path: "wine".to_owned(),
            wine_prefix: data_dir.join("wine-prefix").to_string_lossy().into_owned(),
            last_played: None,
        }
    }
}

fn settings_file() -> PathBuf {
    data_directory().join("ui-settings.conf")
}

pub(crate) fn load_settings() -> std::io::Result<Settings> {
    let mut settings = Settings::defaults();
    let contents = match std::fs::read_to_string(settings_file()) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(settings),
        Err(error) => return Err(error),
    };

    for line in contents.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key {
            "games_directory" => settings.games_directory = value.to_owned(),
            "wine_path" => settings.wine_path = value.to_owned(),
            "wine_prefix" => settings.wine_prefix = value.to_owned(),
            "last_played" if !value.is_empty() => {
                settings.last_played = Some(PathBuf::from(value));
            }
            _ => {}
        }
    }
    Ok(settings)
}

pub(crate) fn save_settings(settings: &Settings) -> std::io::Result<()> {
    std::fs::create_dir_all(data_directory())?;
    let last_played = settings
        .last_played
        .as_ref()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default();
    let contents = format!(
        "games_directory={}\nwine_path={}\nwine_prefix={}\nlast_played={}\n",
        settings.games_directory, settings.wine_path, settings.wine_prefix, last_played
    );
    std::fs::write(settings_file(), contents)
}

pub(crate) fn data_directory() -> PathBuf {
    if cfg!(windows) {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::var_os("USERPROFILE")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("."))
            })
            .join("Xodus")
    } else if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        PathBuf::from(data_home).join("xodus")
    } else {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".local/share/xodus")
    }
}
