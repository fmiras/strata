mod config;
mod hd_wallet;
mod mempool;
mod mnemonic;

use bitcoin::Network;
use clap::{Parser, Subcommand};
use dialoguer::{console::{style, Style}, theme::ColorfulTheme, Select};

use crate::config::{load_config, save_config, Config};
use crate::hd_wallet::{derive_address_from_xpub, derive_bip44_account_xpub, AddressType};
use crate::mempool::fetch_utxos;
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
    /// Generate a receive address
    #[command(alias = "derive")]
    Receive {
        /// Address type: p2wpkh, p2pkh, p2pk, or p2tr (interactive if not specified)
        #[arg(short = 't', long = "type")]
        address_type: Option<String>,
        /// Network to use (mainnet, testnet, signet, regtest)
        #[arg(short, long, default_value = "mainnet")]
        network: String,
        /// Force a specific address index (overrides auto-increment)
        #[arg(short, long)]
        index: Option<u32>,
    },
    /// Manage UTXOs
    #[command(alias = "utxos")]
    Utxo {
        #[command(subcommand)]
        command: UtxoCommands,
    },
}

#[derive(Subcommand)]
enum UtxoCommands {
    /// List UTXOs for all generated addresses
    Ls {
        /// Network to use (mainnet, testnet, signet)
        #[arg(short, long, default_value = "mainnet")]
        network: String,
    },
}

/// Detect script type from address prefix
fn detect_script_type(address: &str) -> &'static str {
    if address.starts_with("bc1q") || address.starts_with("tb1q") {
        "P2WPKH"
    } else if address.starts_with("bc1p") || address.starts_with("tb1p") {
        "P2TR"
    } else if address.starts_with('1') || address.starts_with('m') || address.starts_with('n') {
        "P2PKH"
    } else if address.starts_with('3') || address.starts_with('2') {
        "P2SH"
    } else {
        "Unknown"
    }
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
                        addresses_mainnet: Vec::new(),
                        addresses_testnet: Vec::new(),
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
        Commands::Receive { address_type, network, index } => {
            // Parse or prompt for address type
            let addr_type = match address_type {
                Some(t) => match t.as_str() {
                    "p2wpkh" => AddressType::P2wpkh,
                    "p2pkh" => AddressType::P2pkh,
                    "p2pk" => AddressType::P2pk,
                    "p2tr" => AddressType::P2tr,
                    _ => {
                        eprintln!("Error: Invalid address type. Must be one of: p2wpkh, p2pkh, p2pk, p2tr");
                        std::process::exit(1);
                    }
                },
                None => {
                    // Interactive selection
                    let theme = ColorfulTheme {
                        active_item_style: Style::new().cyan().bold(),
                        active_item_prefix: dialoguer::console::style("  > ".to_string()).cyan().bold(),
                        prompt_style: Style::new().bold(),
                        ..ColorfulTheme::default()
                    };

                    let options = vec![
                        ("P2WPKH", "Native SegWit (bc1q...)", AddressType::P2wpkh),
                        ("P2TR", "Taproot (bc1p...)", AddressType::P2tr),
                        ("P2PKH", "Legacy (1...)", AddressType::P2pkh),
                        ("P2PK", "Raw public key (hex)", AddressType::P2pk),
                    ];

                    let items: Vec<String> = options
                        .iter()
                        .map(|(name, desc, _)| format!("{} - {}", name, desc))
                        .collect();

                    let selection = Select::with_theme(&theme)
                        .with_prompt("Select address type")
                        .items(&items)
                        .default(0)
                        .interact()
                        .unwrap_or_else(|_| {
                            eprintln!("Error: Failed to get user selection");
                            std::process::exit(1);
                        });

                    options[selection].2
                }
            };

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
                Ok(mut config) => {
                    let (xpub, addresses) = match network {
                        Network::Bitcoin => (config.xpub_mainnet.clone(), &mut config.addresses_mainnet),
                        _ => (config.xpub_testnet.clone(), &mut config.addresses_testnet),
                    };

                    // Use forced index or array length as index for next address
                    let derive_index = index.unwrap_or(addresses.len() as u32);

                    match xpub {
                        Some(xpub_str) => {
                            match derive_address_from_xpub(&xpub_str, network, derive_index, addr_type) {
                                Ok(address) => {
                                    println!("{}", address);

                                    // Only update config if not using forced index
                                    if index.is_none() {
                                        addresses.push(address);
                                        if let Err(e) = save_config(&config) {
                                            eprintln!("Warning: Failed to save address to config: {}", e);
                                        }
                                    }
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
        Commands::Utxo { command } => match command {
            UtxoCommands::Ls { network } => {
                // Parse network
                let network = match network.as_str() {
                    "mainnet" => Network::Bitcoin,
                    "testnet" => Network::Testnet,
                    "signet" => Network::Signet,
                    _ => {
                        eprintln!("Error: Invalid network. Must be one of: mainnet, testnet, signet");
                        eprintln!("Note: regtest is not supported by mempool.space API");
                        std::process::exit(1);
                    }
                };

                // Load addresses from config
                match load_config() {
                    Ok(config) => {
                        let addresses = match network {
                            Network::Bitcoin => &config.addresses_mainnet,
                            _ => &config.addresses_testnet,
                        };

                        if addresses.is_empty() {
                            println!("No addresses found. Generate addresses first using 'strata receive'.");
                            return;
                        }

                        println!("Scanning {} addresses for UTXOs...\n", addresses.len());

                        let mut total_utxos = 0;
                        let mut total_value: u64 = 0;

                        for address in addresses {
                            match fetch_utxos(address, network) {
                                Ok(utxos) => {
                                    if !utxos.is_empty() {
                                        let script_type = detect_script_type(address);
                                        println!("Address: {}", address);
                                        println!("{}", "-".repeat(64));

                                        for utxo in &utxos {
                                            let status = if utxo.status.confirmed {
                                                format!("confirmed (block {})", utxo.status.block_height.unwrap_or(0))
                                            } else {
                                                "unconfirmed".to_string()
                                            };

                                            let btc_value = utxo.value as f64 / 100_000_000.0;

                                            println!(
                                                "  TXID:   {}",
                                                utxo.txid
                                            );
                                            println!("  Vout:   {}", utxo.vout);
                                            println!("  Amount: {} sats ({} BTC)",
                                                style(utxo.value).color256(208),
                                                style(format!("{:.8}", btc_value)).color256(208));
                                            println!("  Type:   {}", script_type);
                                            println!("  Status: {}", status);
                                            println!();

                                            total_utxos += 1;
                                            total_value += utxo.value;
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Warning: Failed to fetch UTXOs for {}: {}", address, e);
                                }
                            }
                        }

                        if total_utxos == 0 {
                            println!("No UTXOs found.");
                        } else {
                            let total_btc = total_value as f64 / 100_000_000.0;
                            println!("{}", "=".repeat(64));
                            println!("Total: {} UTXOs, {} sats ({} BTC)",
                                total_utxos,
                                style(total_value).color256(208).bold(),
                                style(format!("{:.8}", total_btc)).color256(208).bold());
                        }
                    }
                    Err(e) => {
                        eprintln!("Error loading config: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        },
    }
}