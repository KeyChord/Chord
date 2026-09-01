// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::{Parser, Subcommand, ValueHint};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "chord", about = "shortcuts reimagined")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Run a JavaScript/TypeScript file with Chord's embedded Bun runtime.
    #[command(alias = "run", trailing_var_arg = true)]
    Bun {
        #[arg(value_name = "SCRIPT", value_hint = ValueHint::FilePath)]
        script: PathBuf,
        #[arg(value_name = "ARG", allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Execute a chord sequence in the running Chord app.
    Chord {
        #[arg(value_name = "SEQUENCE", allow_hyphen_values = true)]
        sequence: String,
    },
    /// Run a shell command as a Chord child process.
    Exec {
        #[arg(value_name = "COMMAND", allow_hyphen_values = true)]
        command: String,
    },
    /// Call an exported function of a JavaScript/TypeScript file.
    #[command(trailing_var_arg = true)]
    RunExport {
        #[arg(value_name = "FILE", value_hint = ValueHint::FilePath)]
        file: PathBuf,
        #[arg(value_name = "EXPORT")]
        export: String,
        #[arg(value_name = "ARG", allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Internal output-discarding wrapper used by runSudoCommand.
    #[command(name = "__elevated-exec", hide = true, trailing_var_arg = true)]
    ElevatedExec {
        #[arg(value_name = "ENCODED_ARG", allow_hyphen_values = true, num_args = 1..)]
        encoded_args: Vec<String>,
    },
    /// Internal detached app launch used for chord execution.
    #[command(name = "__app-chord", hide = true)]
    AppChord {
        #[arg(value_name = "SEQUENCE", allow_hyphen_values = true)]
        sequence: String,
    },
    /// Open Chord's settings.
    #[command(aliases = ["open-settings", "show-settings"])]
    Settings,
    /// Reload Chord's configuration files.
    #[command(name = "reload-configs", alias = "reload-config")]
    ReloadConfigs,
    /// Treat an otherwise unknown command as a chord sequence.
    #[command(external_subcommand)]
    Sequence(Vec<String>),
}

fn main() {
    let cli = Cli::parse();

    // Recover the PATH a GUI launch does not inherit (Finder/Dock give an app a minimal
    // `/usr/bin:/bin:/usr/sbin:/sbin`). `fix_path_env` does that by running `$SHELL -ilc env`,
    // and `-i` sources the user's interactive rc file — commonly hundreds of milliseconds.
    // A CLI invocation already has the right PATH: it was started from that very shell. So pay
    // the cost only when launching the app.
    if cli.command.is_none() {
        let _ = fix_path_env::fix();
    }

    match cli.command {
        Some(Commands::Bun { script, args }) => {
            if let Err(error) = run_cli(script, args) {
                eprintln!("{error:#}");
                std::process::exit(1);
            }
        }
        Some(Commands::Chord { sequence }) => {
            exit_on_error(dispatch_chord(sequence));
        }
        Some(Commands::Exec { command }) => {
            exit_on_error(run_shell_cli(command));
        }
        Some(Commands::RunExport { file, export, args }) => {
            if let Err(error) = run_export_cli(file, export, args) {
                eprintln!("{error:#}");
                std::process::exit(1);
            }
        }
        Some(Commands::ElevatedExec { encoded_args }) => {
            exit_on_error(run_elevated_exec(encoded_args));
        }
        Some(Commands::AppChord { sequence }) => {
            chords_lib::run_app_with_chord(sequence);
        }
        Some(Commands::Settings) => {
            exit_on_error(forward_app_command("settings"));
        }
        Some(Commands::ReloadConfigs) => {
            exit_on_error(forward_app_command("reload-configs"));
        }
        Some(Commands::Sequence(arguments)) => match arguments.as_slice() {
            [sequence] => exit_on_error(dispatch_chord(sequence.clone())),
            _ => exit_on_error(Err(anyhow::anyhow!(
                "a shorthand chord must be a single sequence"
            ))),
        },
        None => chords_lib::run_app(),
    }
}

fn dispatch_chord(sequence: String) -> anyhow::Result<()> {
    let executable = std::env::current_exe()?;
    let mut command = std::process::Command::new(executable);
    command
        .arg("__app-chord")
        .arg(sequence)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    command.spawn()?;
    Ok(())
}

fn exit_on_error(result: anyhow::Result<()>) {
    if let Err(error) = result {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}

fn run_shell_cli(command: String) -> anyhow::Result<()> {
    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(&command)
        .status()?;
    if !status.success() {
        anyhow::bail!("shell command exited with status {status}");
    }
    Ok(())
}

fn run_elevated_exec(encoded_args: Vec<String>) -> anyhow::Result<()> {
    let mut arguments = encoded_args
        .into_iter()
        .map(|argument| decode_elevated_argument(&argument));
    let program = arguments
        .next()
        .transpose()?
        .ok_or_else(|| anyhow::anyhow!("missing elevated program"))?;
    let arguments = arguments.collect::<anyhow::Result<Vec<_>>>()?;
    let status = std::process::Command::new(program)
        .args(arguments)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;
    if !status.success() {
        anyhow::bail!("elevated command exited with status {status}");
    }
    Ok(())
}

fn decode_elevated_argument(encoded: &str) -> anyhow::Result<String> {
    let encoded = encoded
        .strip_prefix('x')
        .ok_or_else(|| anyhow::anyhow!("invalid elevated argument"))?;
    if encoded.len() % 2 != 0 {
        anyhow::bail!("invalid elevated argument");
    }
    let bytes = encoded
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair)?;
            Ok(u8::from_str_radix(pair, 16)?)
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(String::from_utf8(bytes)?)
}

#[cfg(target_os = "macos")]
fn forward_app_command(command: &str) -> anyhow::Result<()> {
    let url = format!("chord:{command}");
    let status = std::process::Command::new("open")
        .arg("--background")
        .arg(&url)
        .status()?;
    if !status.success() {
        anyhow::bail!("failed to open {url}");
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn forward_app_command(_command: &str) -> anyhow::Result<()> {
    anyhow::bail!("Chord app commands are currently supported only on macOS")
}

fn run_cli(script: PathBuf, args: Vec<String>) -> anyhow::Result<()> {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(chords_lib::run_script_with_args(script, args))
    })
    .join()
    .unwrap()
}

fn run_export_cli(file: PathBuf, export: String, args: Vec<String>) -> anyhow::Result<()> {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(chords_lib::run_script_export(file, export, args))
    })
    .join()
    .unwrap()
}

#[cfg(test)]
mod tests {
    use super::{Cli, Commands, decode_elevated_argument};
    use clap::Parser;

    #[test]
    fn parses_bun_script_arguments() {
        let cli = Cli::try_parse_from(["chord", "bun", "script.ts", "--flag", "value"]).unwrap();
        let Some(Commands::Bun { script, args }) = cli.command else {
            panic!("expected bun command");
        };
        assert_eq!(script.to_string_lossy(), "script.ts");
        assert_eq!(args, ["--flag", "value"]);
    }

    #[test]
    fn parses_explicit_chord() {
        let cli = Cli::try_parse_from(["chord", "chord", "fq"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Chord { sequence }) if sequence == "fq"
        ));
    }

    #[test]
    fn parses_shell_command() {
        let cli = Cli::try_parse_from(["chord", "exec", "printf hello"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Exec { command }) if command == "printf hello"
        ));
    }

    #[test]
    fn parses_shorthand_chord() {
        let cli = Cli::try_parse_from(["chord", "fq"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Sequence(arguments)) if arguments == ["fq"]
        ));
    }

    #[test]
    fn parses_and_decodes_elevated_helper_arguments() {
        let cli = Cli::try_parse_from(["chord", "__elevated-exec", "x2f62696e2f6563686f", "x6869"])
            .unwrap();
        let Some(Commands::ElevatedExec { encoded_args }) = cli.command else {
            panic!("expected elevated helper command");
        };
        assert_eq!(
            decode_elevated_argument(&encoded_args[0]).unwrap(),
            "/bin/echo"
        );
        assert_eq!(decode_elevated_argument(&encoded_args[1]).unwrap(), "hi");
    }
}
