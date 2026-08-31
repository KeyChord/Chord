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
    /// Run a JavaScript/TypeScript file.
    Run {
        #[arg(value_name = "FILE", value_hint = ValueHint::FilePath)]
        file: PathBuf,
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
        Some(Commands::Run { file }) => {
            if let Err(error) = run_cli(file) {
                eprintln!("{error:#}");
                std::process::exit(1);
            }
        }
        Some(Commands::RunExport {
            file,
            export,
            args,
        }) => {
            if let Err(error) = run_export_cli(file, export, args) {
                eprintln!("{error:#}");
                std::process::exit(1);
            }
        }
        None => chords_lib::run_app(),
    }
}

fn run_cli(file: PathBuf) -> anyhow::Result<()> {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(chords_lib::run_script(file))
    })
    .join()
    .unwrap()
}

fn run_export_cli(
    file: PathBuf,
    export: String,
    args: Vec<String>,
) -> anyhow::Result<()> {
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
