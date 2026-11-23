//! Linux and Windows launch Bun as a sidecar and share the package supervisor with macOS.

use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};
use std::process::Command;

mod host;
pub mod lifecycle;
pub(crate) mod package_worker;
pub(crate) mod protocol;

const RUNTIME: &str = include_str!("portable-runtime.js");
const BUN: &str = if cfg!(windows) {
    "chord-bun.exe"
} else {
    "chord-bun"
};

fn bun_command(executable: &Path) -> Result<Command> {
    let bundled = executable.with_file_name(BUN);
    let mut command = if bundled.is_file() {
        Command::new(bundled)
    } else if cfg!(debug_assertions) {
        Command::new("bun")
    } else {
        anyhow::bail!(
            "Chord's bundled Bun runtime is missing at {}",
            bundled.display()
        );
    };
    command.arg("--no-install").arg("--eval").arg(RUNTIME);
    command.env("CHORD_TARGET_TRIPLE", env!("CHORD_TARGET_TRIPLE"));
    Ok(command)
}

pub(super) fn worker_command(executable: &Path, address: &str, token: &str) -> Result<Command> {
    let mut command = bun_command(executable)?;
    command.args(["worker", address, token]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    Ok(command)
}

pub async fn run_standalone_module_with_args(path: &Path, args: Vec<String>) -> Result<()> {
    standalone("script", path, None, args).await
}

pub async fn run_standalone_export(path: &Path, export: String, args: Vec<String>) -> Result<()> {
    standalone("export", path, Some(export), args).await
}

async fn standalone(
    mode: &str,
    path: &Path,
    export: Option<String>,
    args: Vec<String>,
) -> Result<()> {
    let path = path
        .canonicalize()
        .with_context(|| format!("could not open {}", path.display()))?;
    let executable: PathBuf = std::env::current_exe()?;
    let mut command = bun_command(&executable)?;
    command.arg(mode).arg(path);
    if let Some(export) = export {
        command.arg(export);
    }
    let status = tokio::process::Command::from(command)
        .args(args)
        .kill_on_drop(true)
        .status()
        .await
        .context("could not start Chord's Bun runtime")?;
    ensure!(status.success(), "Bun exited with {status}");
    Ok(())
}
