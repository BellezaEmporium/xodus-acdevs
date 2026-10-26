use std::path::PathBuf;
use std::process::Command;

pub fn cli_command() -> Command {
    let cli_name = if cfg!(windows) {
        "xodus-cli.exe"
    } else {
        "xodus-cli"
    };
    let adjacent_cli = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|parent| parent.join(cli_name)))
        .filter(|path| path.is_file());
    Command::new(adjacent_cli.unwrap_or_else(|| PathBuf::from(cli_name)))
}

pub fn run_cli_login() -> Result<(), String> {
    let cli_name = if cfg!(windows) {
        "xodus-cli.exe"
    } else {
        "xodus-cli"
    };

    let mut command = cli_command();

    let status = command
        .arg("login")
        .status()
        .map_err(|error| {
            format!(
                "Could not start {cli_name} login: {error}. Put xodus-cli next to the GUI executable or on PATH."
            )
        })?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("xodus-cli login exited with status {status}."))
    }
}