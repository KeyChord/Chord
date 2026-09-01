use crate::api::{ApiImpl, AppResult, ChordCommandOutput};
use crate::process::{COMMAND_OUTPUT_LIMIT, COMMAND_TIMEOUT, bounded_output};
use anyhow::{Context, Result, bail};
use tokio::process::Command;

pub async fn run_chord_command(_api: ApiImpl, command: String) -> AppResult<ChordCommandOutput> {
    Ok(run(command).await?)
}

async fn run(command: String) -> Result<ChordCommandOutput> {
    let args = command_arguments(&command)?;
    let executable = std::env::current_exe().context("failed to locate the Chord executable")?;
    let output = bounded_output(
        Command::new(executable).args(args),
        COMMAND_OUTPUT_LIMIT,
        COMMAND_TIMEOUT,
    )
    .await
    .context("failed to run the Chord command")?;

    Ok(ChordCommandOutput {
        exit_code: output.status.code(),
        stderr: output.stderr.to_lossy_text(),
        stdout: output.stdout.to_lossy_text(),
    })
}

fn command_arguments(command: &str) -> Result<Vec<String>> {
    let mut args = shlex::split(command).context("command contains an unterminated quote")?;
    if args.first().is_some_and(|arg| arg == "chord") {
        args.remove(0);
    }
    if args.is_empty() {
        bail!("enter a Chord command, such as `help`");
    }
    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::command_arguments;

    #[test]
    fn parses_command_arguments() {
        assert_eq!(
            command_arguments("run-export 'my script.ts' main one").unwrap(),
            ["run-export", "my script.ts", "main", "one"]
        );
        assert_eq!(command_arguments("chord help").unwrap(), ["help"]);
    }

    #[test]
    fn rejects_empty_and_unterminated_commands() {
        assert!(command_arguments("").is_err());
        assert!(command_arguments("chord").is_err());
        assert!(command_arguments("run '").is_err());
    }
}
