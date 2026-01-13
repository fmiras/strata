mod mnemonic;

use clap::{Parser, Subcommand};

use crate::mnemonic::generate_and_save_mnemonic;

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
        /// Display the generated mnemonic in console output
        #[arg(short, long)]
        show: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Generate { words, show } => {
            match generate_and_save_mnemonic(*words) {
                Ok(mnemonic) => {
                    let phrase = mnemonic.words().collect::<Vec<_>>().join(" ");
                    if *show {
                        println!("⚠️ WARNING: This is for educational purposes only. Do not use this mnemonic in a production environment.");
                        println!("{}", phrase);
                    }
                }
                Err(e) => {
                    eprintln!("Error generating mnemonic: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}