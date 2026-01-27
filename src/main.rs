mod config;
mod hd_wallet;
mod mempool;
mod mnemonic;
mod qr;

use arboard::Clipboard;
use bitcoin::address::Address;
use bitcoin::blockdata::locktime::absolute::LockTime;
use bitcoin::blockdata::script::ScriptBuf;
use bitcoin::blockdata::transaction::{OutPoint, Transaction, TxIn, TxOut, Version};
use bitcoin::consensus::encode::serialize_hex;
use bitcoin::hashes::Hash;
use bitcoin::secp256k1::{Message, Secp256k1};
use bitcoin::sighash::SighashCache;
use bitcoin::{EcdsaSighashType, Network, Sequence, Txid, Witness};
use clap::{Parser, Subcommand};
use dialoguer::{console::{style, Style}, theme::ColorfulTheme, Select};
use std::str::FromStr;

use crate::config::{load_config, save_config, Config};
use crate::hd_wallet::{derive_address_from_xpub, derive_bip44_account_xpub, derive_private_key, derive_public_key, AddressType};
use crate::mempool::{broadcast_tx, fetch_tx, fetch_utxos};
use crate::mnemonic::{generate_and_save_mnemonic, load_mnemonic};
use crate::qr::print_qr_code;

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
        /// Network to use (mainnet, testnet, signet)
        #[arg(short, long, default_value = "mainnet")]
        network: String,
    },
    /// Generate a receive address
    #[command(alias = "derive")]
    Receive {
        /// Address type: p2wpkh, p2pkh, p2pk, or p2tr (interactive if not specified)
        #[arg(short = 't', long = "type")]
        address_type: Option<String>,
        /// Network to use (mainnet, testnet, signet)
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
    /// Send bitcoin to an address (supports P2PKH and P2WPKH)
    #[command(alias = "spend")]
    Send {
        /// Input UTXO in format TXID:VOUT
        #[arg(short, long)]
        input: String,
        /// Destination address
        #[arg(short, long)]
        to: String,
        /// Total fee in sats
        #[arg(short, long)]
        fee: u64,
        /// Network to use (mainnet, testnet, signet)
        #[arg(short, long, default_value = "mainnet")]
        network: String,
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

/// Format a number with comma separators (e.g., 1000000 -> "1,000,000")
fn format_with_commas<T: std::fmt::Display>(n: T) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*c);
    }
    result
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

/// Parse network string to Network enum
/// 
/// # Arguments
/// * `network_str` - Network string (mainnet, testnet, signet)
/// 
/// # Returns
/// * `Ok(Network)` - Parsed network
/// * `Err(String)` - Error message if network is invalid
fn parse_network(network_str: &str) -> Result<Network, String> {
    match network_str {
        "mainnet" => Ok(Network::Bitcoin),
        "testnet" => Ok(Network::Testnet),
        "signet" => Ok(Network::Signet),
        _ => {
            Err("Error: Invalid network. Must be one of: mainnet, testnet, signet".to_string())
        }
    }
}


/// Copy text to clipboard and return success status
fn copy_to_clipboard(text: &str) -> bool {
    match Clipboard::new() {
        Ok(mut clipboard) => clipboard.set_text(text).is_ok(),
        Err(_) => false,
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
            let network = match parse_network(&network) {
                Ok(n) => n,
                Err(e) => {
                    eprintln!("{}", e);
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
            let network = match parse_network(&network) {
                Ok(n) => n,
                Err(e) => {
                    eprintln!("{}", e);
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
                                    // Print QR code (skip for P2PK since it's just a hex pubkey)
                                    if addr_type != AddressType::P2pk {
                                        print_qr_code(&address);
                                    }

                                    // Print address
                                    println!("{}", address);

                                    // Copy to clipboard and show status
                                    if copy_to_clipboard(&address) {
                                        println!("{}", style("(copied to clipboard)").cyan());
                                    }

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
                let network = match parse_network(&network) {
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("{}", e);
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

                        println!(
                            "{} {} addresses for UTXOs...\n",
                            style("Scanning").dim(),
                            addresses.len()
                        );

                        let mut total_utxos = 0;
                        let mut total_value: u64 = 0;

                        for address in addresses {
                            match fetch_utxos(address, network) {
                                Ok(utxos) => {
                                    if !utxos.is_empty() {
                                        let script_type = detect_script_type(address);

                                        // Address header with type badge
                                        println!(
                                            "  {} {}",
                                            style(address).bold(),
                                            style(format!("[{}]", script_type)).dim()
                                        );

                                        let utxo_count = utxos.len();
                                        for (i, utxo) in utxos.iter().enumerate() {
                                            let is_last = i == utxo_count - 1;
                                            let branch = if is_last { "└─" } else { "├─" };
                                            let continuation = if is_last { "   " } else { "│  " };

                                            // Status with symbol and color
                                            let status_display = if utxo.status.confirmed {
                                                let block = utxo.status.block_height.unwrap_or(0);
                                                format!(
                                                    "{} {}",
                                                    style("✔").green(),
                                                    style(format!("block {}", format_with_commas(block))).dim()
                                                )
                                            } else {
                                                format!("{} {}", style("◌").yellow(), style("pending").yellow())
                                            };

                                            // Format amounts
                                            let btc_value = utxo.value as f64 / 100_000_000.0;
                                            let sats_formatted = format_with_commas(utxo.value);

                                            // Truncate TXID: first 8 + ... + last 8
                                            let txid_short = if utxo.txid.len() > 20 {
                                                format!("{}...{}", &utxo.txid[..8], &utxo.txid[utxo.txid.len()-8..])
                                            } else {
                                                utxo.txid.clone()
                                            };

                                            // UTXO line with box drawing
                                            println!(
                                                "  {} {} {}",
                                                style(branch).dim(),
                                                style(format!("{} sats", sats_formatted)).cyan().bold(),
                                                style(format!("({:.8} BTC)", btc_value)).dim()
                                            );

                                            // Details indented under the UTXO
                                            println!(
                                                "  {}  {} {}:{} {}",
                                                style(continuation).dim(),
                                                style("txid").dim(),
                                                txid_short,
                                                utxo.vout,
                                                status_display
                                            );

                                            total_utxos += 1;
                                            total_value += utxo.value;
                                        }
                                        println!();
                                    }
                                }
                                Err(e) => {
                                    eprintln!(
                                        "  {} Failed to fetch UTXOs for {}: {}",
                                        style("!").yellow(),
                                        address,
                                        e
                                    );
                                }
                            }
                        }

                        if total_utxos == 0 {
                            println!("{}", style("No UTXOs found.").dim());
                        } else {
                            let total_btc = total_value as f64 / 100_000_000.0;
                            println!();
                            println!("{}", style("─".repeat(50)).dim());
                            println!();
                            println!(
                                "  {} {} {}  {}",
                                style("Total").dim(),
                                style(format!("{} sats", format_with_commas(total_value))).cyan().bold(),
                                style(format!("({:.8} BTC)", total_btc)).dim(),
                                style(format!("{} UTXOs", total_utxos)).dim()
                            );
                            println!();
                        }
                    }
                    Err(e) => {
                        eprintln!("Error loading config: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        },
        Commands::Send { input, to, fee, network } => {
            // Parse network
            let network = match parse_network(&network) {
                Ok(n) => n,
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            };

            // Parse input (TXID:VOUT)
            let parts: Vec<&str> = input.split(':').collect();
            if parts.len() != 2 {
                eprintln!("Error: Invalid input format. Expected TXID:VOUT (e.g., 5702c1...:0)");
                std::process::exit(1);
            }
            let txid_str = parts[0];
            let vout: u32 = match parts[1].parse() {
                Ok(v) => v,
                Err(_) => {
                    eprintln!("Error: Invalid vout. Must be a number.");
                    std::process::exit(1);
                }
            };

            // Fetch the previous transaction to get the scriptPubKey and value
            println!("Fetching transaction {}...", txid_str);
            let prev_tx = match fetch_tx(txid_str, network) {
                Ok(tx) => tx,
                Err(e) => {
                    eprintln!("Error fetching transaction: {}", e);
                    std::process::exit(1);
                }
            };

            let prev_output = match prev_tx.vout.get(vout as usize) {
                Some(out) => out,
                None => {
                    eprintln!("Error: Output index {} not found in transaction", vout);
                    std::process::exit(1);
                }
            };

            // Check if this is a supported output type
            let script_type = &prev_output.scriptpubkey_type;
            if script_type != "p2pkh" && script_type != "v0_p2wpkh" {
                eprintln!("Error: Only P2PKH and P2WPKH outputs are supported. This output is: {}", script_type);
                std::process::exit(1);
            }

            let input_value = prev_output.value;
            let prev_scriptpubkey = ScriptBuf::from_hex(&prev_output.scriptpubkey)
                .expect("Invalid scriptPubKey hex");

            // Load config to find which address owns this UTXO
            let config = match load_config() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error loading config: {}", e);
                    std::process::exit(1);
                }
            };

            let addresses = match network {
                Network::Bitcoin => &config.addresses_mainnet,
                _ => &config.addresses_testnet,
            };

            // Find the address index that owns this UTXO
            let mut found_index: Option<u32> = None;
            for (idx, addr) in addresses.iter().enumerate() {
                let addr_script_type = detect_script_type(addr);
                // Check P2PKH and P2WPKH addresses
                if addr_script_type == "P2PKH" || addr_script_type == "P2WPKH" {
                    let addr_parsed = Address::from_str(addr)
                        .expect("Invalid address in config")
                        .require_network(network)
                        .expect("Address network mismatch");

                    if addr_parsed.script_pubkey() == prev_scriptpubkey {
                        found_index = Some(idx as u32);
                        println!("Found UTXO owner: {} (index {}, type {})", addr, idx, addr_script_type);
                        break;
                    }
                }
            }

            let address_index = match found_index {
                Some(idx) => idx,
                None => {
                    eprintln!("Error: Could not find the address that owns this UTXO in your wallet.");
                    std::process::exit(1);
                }
            };

            // Load mnemonic and derive private key
            let mnemonic = match load_mnemonic() {
                Ok(Some(m)) => m,
                Ok(None) => {
                    eprintln!("Error: No mnemonic found in keychain. Generate one first using 'strata generate'.");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error loading mnemonic: {}", e);
                    std::process::exit(1);
                }
            };

            let secret_key = match derive_private_key(&mnemonic, network, address_index) {
                Ok(sk) => sk,
                Err(e) => {
                    eprintln!("Error deriving private key: {}", e);
                    std::process::exit(1);
                }
            };

            // Parse destination address
            let dest_address = match Address::from_str(to) {
                Ok(addr) => match addr.require_network(network) {
                    Ok(a) => a,
                    Err(_) => {
                        eprintln!("Error: Destination address is not valid for this network");
                        std::process::exit(1);
                    }
                },
                Err(e) => {
                    eprintln!("Error parsing destination address: {}", e);
                    std::process::exit(1);
                }
            };

            // Dust threshold for outputs (546 sats)
            const DUST_THRESHOLD: u64 = 546;

            // Validate: input must cover fee + dust threshold at minimum
            let min_required = fee + DUST_THRESHOLD;
            if input_value < min_required {
                eprintln!("Error: Input value ({} sats) is insufficient.", input_value);
                eprintln!("  Required: {} sats (fee) + {} sats (dust threshold) = {} sats minimum",
                    fee, DUST_THRESHOLD, min_required);
                std::process::exit(1);
            }

            let output_value = input_value - fee;

            println!("\nTransaction Details:");
            println!("{}", "-".repeat(64));
            println!("Input:  {}:{}", txid_str, vout);
            println!("Value:  {} sats", style(input_value).color256(208));
            println!("To:     {}", to);
            println!("Amount: {} sats", style(output_value).color256(208));
            println!("Fee:    {} sats", style(*fee).color256(208));
            println!("{}", "-".repeat(64));

            // Build the transaction
            let txid = Txid::from_str(txid_str).expect("Invalid transaction id");
            let outpoint = OutPoint { txid, vout };

            let tx_in = TxIn {
                previous_output: outpoint,
                script_sig: ScriptBuf::new(), // Will be filled after signing
                sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
                witness: Witness::new(),
            };

            let tx_out = TxOut {
                value: bitcoin::Amount::from_sat(output_value),
                script_pubkey: dest_address.script_pubkey(),
            };

            let mut unsigned_tx = Transaction {
                version: Version::TWO,
                lock_time: LockTime::ZERO,
                input: vec![tx_in],
                output: vec![tx_out],
            };

            // Get the public key for signing
            let public_key = match derive_public_key(&mnemonic, network, address_index) {
                Ok(pk) => pk,
                Err(e) => {
                    eprintln!("Error deriving public key: {}", e);
                    std::process::exit(1);
                }
            };

            let secp = Secp256k1::new();

            if script_type == "v0_p2wpkh" {
                // Sign P2WPKH (SegWit v0)
                let mut sighash_cache = SighashCache::new(&mut unsigned_tx);

                let sighash = sighash_cache
                    .p2wpkh_signature_hash(
                        0,
                        &prev_scriptpubkey,
                        bitcoin::Amount::from_sat(input_value),
                        EcdsaSighashType::All,
                    )
                    .expect("Failed to compute segwit sighash");

                let message = Message::from_digest(*sighash.as_byte_array());
                let signature = secp.sign_ecdsa(&message, &secret_key);

                // Build witness: [signature, pubkey]
                let mut sig_bytes = signature.serialize_der().to_vec();
                sig_bytes.push(EcdsaSighashType::All.to_u32() as u8);

                let mut witness = Witness::new();
                witness.push(&sig_bytes);
                witness.push(&public_key.to_bytes());

                *sighash_cache.witness_mut(0).unwrap() = witness;
            } else {
                // Sign P2PKH (Legacy)
                let sighash_cache = SighashCache::new(&unsigned_tx);

                let sighash = sighash_cache
                    .legacy_signature_hash(0, &prev_scriptpubkey, EcdsaSighashType::All.to_u32())
                    .expect("Failed to compute sighash");

                let message = Message::from_digest(*sighash.as_byte_array());
                let signature = secp.sign_ecdsa(&message, &secret_key);

                // Build scriptSig: <signature> <pubkey>
                let mut sig_bytes = signature.serialize_der().to_vec();
                sig_bytes.push(EcdsaSighashType::All.to_u32() as u8);

                let script_sig = bitcoin::blockdata::script::Builder::new()
                    .push_slice::<&bitcoin::script::PushBytes>(sig_bytes.as_slice().try_into().expect("signature too long"))
                    .push_slice::<&bitcoin::script::PushBytes>(public_key.to_bytes().as_slice().try_into().expect("pubkey too long"))
                    .into_script();

                unsigned_tx.input[0].script_sig = script_sig;
            }

            // Serialize and broadcast
            let tx_hex = serialize_hex(&unsigned_tx);

            println!("\nRaw Transaction:");
            println!("{}", tx_hex);
            println!();

            println!("Broadcasting transaction...");
            match broadcast_tx(&tx_hex, network) {
                Ok(txid) => {
                    println!("\nTransaction broadcast successfully!");
                    println!("TXID: {}", style(txid).color256(208).bold());
                }
                Err(e) => {
                    eprintln!("\nError broadcasting transaction: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}