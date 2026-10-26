// Gets the current installation event for a game. Based on xodus-cli's "streaming/install" command.
pub enum InstallEvent {
    Started {
        total_bytes: u64,
    },
    Progress {
        completed_bytes: u64,
        total_bytes: u64,
    },
    Status {
        message: String,
    },
    Completed,
    Failed {
        message: String,
    },
}