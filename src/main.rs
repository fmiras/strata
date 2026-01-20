mod config;
mod hd_wallet;
mod mnemonic;

use bitcoin::Network;
use clap::{Parser, Subcommand};

use crate::config::{load_config, save_config, Config};
use crate::hd_wallet::{derive_address_from_xpub, derive_bip44_account_xpub};
use crate::mnemonic::{generate_and_save_mnemonic, load_mnemonic};

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
    /// Show the extended public key (xpub) of the saved mnemonic
    Xpub {
        /// Network to use (mainnet, testnet, signet, regtest)
        #[arg(short, long, default_value = "mainnet")]
        network: String,
    },
    /// Generate a receive address (legacy P2PKH)
    Receive {
        /// Network to use (mainnet, testnet, signet, regtest)
        #[arg(short, long, default_value = "mainnet")]
        network: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Generate { words, show } => {
            match generate_and_save_mnemonic(*words) {
                Ok(mnemonic) => {
                    // Derive and save xpubs to config file
                    let xpub_mainnet = derive_bip44_account_xpub(&mnemonic, Network::Bitcoin)
                        .expect("Failed to derive mainnet xpub");
                    let xpub_testnet = derive_bip44_account_xpub(&mnemonic, Network::Testnet)
                        .expect("Failed to derive testnet xpub");

                    let config = Config {
                        xpub_mainnet: Some(xpub_mainnet),
                        xpub_testnet: Some(xpub_testnet),
                    };

                    if let Err(e) = save_config(&config) {
                        eprintln!("Warning: Failed to save xpub to config: {}", e);
                    }

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
        Commands::Xpub { network } => {
            // Parse network
            let network = match network.as_str() {
                "mainnet" => Network::Bitcoin,
                "testnet" => Network::Testnet,
                "signet" => Network::Signet,
                "regtest" => Network::Regtest,
                _ => {
                    eprintln!("Error: Invalid network. Must be one of: mainnet, testnet, signet, regtest");
                    std::process::exit(1);
                }
            };

            // Load mnemonic from keychain
            match load_mnemonic() {
                Ok(Some(mnemonic)) => {
                    match derive_bip44_account_xpub(&mnemonic, network) {
                        Ok(xpub) => {
                            println!("{}", xpub);
                        }
                        Err(e) => {
                            eprintln!("Error deriving xpub: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                Ok(None) => {
                    eprintln!("Error: No mnemonic found in keychain. Generate one first using 'strata generate'.");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error loading mnemonic from keychain: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Receive { network } => {
            // Parse network
            let network = match network.as_str() {
                "mainnet" => Network::Bitcoin,
                "testnet" => Network::Testnet,
                "signet" => Network::Signet,
                "regtest" => Network::Regtest,
                _ => {
                    eprintln!("Error: Invalid network. Must be one of: mainnet, testnet, signet, regtest");
                    std::process::exit(1);
                }
            };

            // Load xpub from config file (no keychain access needed)
            match load_config() {
                Ok(config) => {
                    let xpub = match network {
                        Network::Bitcoin => config.xpub_mainnet,
                        _ => config.xpub_testnet,
                    };

                    match xpub {
                        Some(xpub_str) => {
                            match derive_address_from_xpub(&xpub_str, network) {
                                Ok(address) => {
                                    println!("{}", address);
                                }
                                Err(e) => {
                                    eprintln!("Error deriving address: {}", e);
                                    std::process::exit(1);
                                }
                            }
                        }
                        None => {
                            eprintln!("Error: No xpub found in config. Generate a wallet first using 'strata generate'.");
                            std::process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error loading config: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}