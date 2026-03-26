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
    Run {
        #[arg(value_name = "FILE", value_hint = ValueHint::FilePath)]
        file: PathBuf,
    },
}

fn main() {
    let _ = fix_path_env::fix();
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Run { file }) => {
            if let Err(error) = run_cli(file) {
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
