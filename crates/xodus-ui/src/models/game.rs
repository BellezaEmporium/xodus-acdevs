use std::path::PathBuf;

// WIP, BellezaEmporium 2026.
// Part of the Xodus UI & the Xodus Game Launcher, for Linux & macOS.

#[derive(Clone)]
pub struct Game {
    pub(crate) path: PathBuf,
    pub(crate) title: String,
    pub(crate) executable: Option<String>,
    pub(crate) provider: GameProvider,
}

#[derive(Clone, PartialEq, Eq)]
pub enum GameProvider {
    Xodus,
    Gog,
    Ea,
    Steam,
    Ubisoft,
    Riot,
    Other(String),
}

impl GameProvider {
    pub(crate) fn from_manifest(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "gog" | "comet" => Self::Gog,
            "ea" | "maxima" => Self::Ea,
            "steam" => Self::Steam,
            "ubisoft" | "ubisoft connect" => Self::Ubisoft,
            "riot" | "riot client" => Self::Riot,
            "" | "xodus" | "xbox" => Self::Xodus,
            _ => Self::Other(value.trim().to_owned()),
        }
    }

    pub(crate) fn ipc_name(&self) -> Option<String> {
        match self {
            Self::Xodus => None,
            Self::Gog => Some("comet - GOG".to_owned()),
            Self::Ea => Some("maxima - EA".to_owned()),
            Self::Steam => Some("steam - Steam".to_owned()),
            Self::Ubisoft => Some("ubisoft connect - Ubisoft".to_owned()),
            Self::Riot => Some("riot client - Riot".to_owned()),
            Self::Other(provider) => Some(provider.clone()),
        }
    }

    pub(crate) fn display_name(&self) -> &str {
        match self {
            Self::Xodus => "Xodus",
            Self::Gog => "GOG",
            Self::Ea => "EA",
            Self::Steam => "Steam",
            Self::Ubisoft => "Ubisoft",
            Self::Riot => "Riot",
            Self::Other(provider) => provider,
        }
    }
}
