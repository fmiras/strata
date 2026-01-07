mod generate_mnemonic;

use clap::{Parser, Subcommand};
use generate_mnemonic::generate_mnemonic;

#[derive(Parser)]
#[command(name = "strata")]
#[command(about = "Strata Wallet: An Archaeological Bitcoin Wallet", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new BIP-39 mnemonic and master seed
    Generate {
        #[arg(short, long, default_value_t = 12)]
        words: usize,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Generate { words } => {
            generate_mnemonic(*words);
        }
    }
}