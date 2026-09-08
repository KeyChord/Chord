use anyhow::{Context, Result, bail};
use std::{fs, io::ErrorKind, path::Path};

pub fn is_installed(destination: &Path, executable: &Path) -> Result<bool> {
    match fs::read_link(destination) {
        Ok(target) => Ok(target == executable),
        Err(error) if matches!(error.kind(), ErrorKind::NotFound | ErrorKind::InvalidInput) => {
            Ok(false)
        }
        Err(error) => Err(error).context("Could not inspect the command line installation"),
    }
}

pub fn install(executable: &Path, destination: &Path) -> Result<()> {
    if is_installed(destination, executable)? {
        return Ok(());
    }
    // Include dangling symlinks in the conflict check. Never replace another installation.
    match fs::symlink_metadata(destination) {
        Ok(_) => bail!(
            "{} already exists. Move or remove it before installing this version of Chord.",
            destination.display()
        ),
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let parent = destination
        .parent()
        .context("Missing installation directory")?;
    match fs::create_dir_all(parent)
        .and_then(|()| std::os::unix::fs::symlink(executable, destination))
    {
        Ok(()) => return Ok(()),
        Err(error) if error.kind() == ErrorKind::PermissionDenied => {}
        Err(error) => return Err(error).context("Could not install the command line tool"),
    }

    // AppleScript receives the shell command as an argument, with every path shell-quoted.
    let quote = |path: &Path| -> Result<String> {
        Ok(shlex::try_quote(
            path.to_str()
                .context("Installation path is not valid UTF-8")?,
        )?
        .into_owned())
    };
    let destination = quote(destination)?;
    let command = format!(
        "/bin/mkdir -p {} && test ! -e {destination} && test ! -L {destination} && /bin/ln -sh {} {destination}",
        quote(parent)?,
        quote(executable)?,
    );
    let output = std::process::Command::new("/usr/bin/osascript")
        .args([
            "-e",
            "on run argv\ndo shell script (item 1 of argv) with administrator privileges\nend run",
            &command,
        ])
        .output()
        .context("Could not request administrator access")?;
    if !output.status.success() {
        bail!(
            "Could not install the command line tool: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installs_idempotently_and_preserves_conflicts() {
        let root = std::env::temp_dir().join(format!("chord-cli-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let executable = root.join("Chord Dev's executable");
        fs::write(&executable, "app").unwrap();
        let destination = root.join("bin/chordd");
        assert!(!is_installed(&destination, &executable).unwrap());
        install(&executable, &destination).unwrap();
        install(&executable, &destination).unwrap();
        assert!(is_installed(&destination, &executable).unwrap());
        fs::remove_file(&destination).unwrap();
        fs::write(&destination, "existing command").unwrap();
        assert!(install(&executable, &destination).is_err());
        assert_eq!(
            fs::read_to_string(&destination).unwrap(),
            "existing command"
        );
        fs::remove_file(&destination).unwrap();
        std::os::unix::fs::symlink(root.join("missing"), &destination).unwrap();
        assert!(install(&executable, &destination).is_err());
        assert_eq!(fs::read_link(&destination).unwrap(), root.join("missing"));
        fs::remove_dir_all(root).unwrap();
    }
}
