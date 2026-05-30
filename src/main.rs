use clap::{Parser, Subcommand};
use niri_bjuwk::error::BjuwkResult;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "niri-bjuwk", about = "Save and restore niri window layouts")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Save {
        #[arg(short, long)]
        snapshot: Option<PathBuf>,
    },
    Restore {
        #[arg(short, long)]
        snapshot: Option<PathBuf>,
        #[arg(short = 'c', long)]
        match_config: Option<PathBuf>,
        #[arg(short = 'n', long)]
        dry_run: bool,
        #[arg(long)]
        no_workspace_rename: bool,
    },
}

fn main() -> BjuwkResult<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Save { snapshot } => {
            println!("Saving window layout...");
            niri_bjuwk::save::execute(snapshot)?;
        }
        Command::Restore {
            snapshot,
            match_config,
            dry_run,
            no_workspace_rename,
        } => {
            if dry_run {
                println!("(dry-run) Restoring window layout...");
            } else {
                println!("Restoring window layout...");
            }
            niri_bjuwk::restore::execute(snapshot, match_config, dry_run, no_workspace_rename)?;
        }
    }

    Ok(())
}
