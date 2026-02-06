mod commands;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "weave", about = "Entity-level semantic merge for Git")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Configure the current Git repo to use weave as merge driver
    Setup {
        /// Path to weave-driver binary (auto-detected if omitted)
        #[arg(long)]
        driver: Option<String>,
    },
    /// Preview what a merge between branches would look like
    Preview {
        /// The branch to merge into HEAD
        branch: String,
        /// Optional: preview a specific file only
        #[arg(long)]
        file: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Setup { ref driver } => {
            commands::setup::run(driver.as_deref())
        }
        Commands::Preview { ref branch, ref file } => {
            commands::preview::run(branch, file.as_deref())
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
