use bip32::{DerivationPath, Prefix, XPrv};
use bip39::Mnemonic;
use bitcoin::address::Address;
use bitcoin::key::CompressedPublicKey;
use bitcoin::Network;
use std::error::Error;
use std::str::FromStr;

/// Derives the extended public key (xpub) from a mnemonic.
/// Uses BIP39 to convert mnemonic to seed, then BIP32 to derive the master xpub.
pub fn derive_xpub(mnemonic: &Mnemonic, network: Network) -> Result<String, Box<dyn Error>> {
    // Convert mnemonic to seed (BIP39)
    let seed = mnemonic.to_seed("");
    
    // Derive master extended private key from seed (BIP32)
    let master_key: XPrv = XPrv::new(seed)?;
    
    // Get the extended public key from the master private key
    let xpub = master_key.public_key();
    
    // Determine the prefix based on network
    let prefix = match network {
        Network::Bitcoin => Prefix::XPUB,
        Network::Testnet | Network::Signet | Network::Regtest | Network::Testnet4 => Prefix::TPUB,
    };
    
    // Format as xpub string (base58 encoded) with network prefix
    Ok(xpub.to_string(prefix))
}

/// Derives a legacy P2PKH address from a mnemonic using BIP44 derivation.
/// Path: m/44'/coin'/0'/0/0 where coin is 0 for mainnet, 1 for testnet.
pub fn derive_legacy_address(mnemonic: &Mnemonic, network: Network) -> Result<String, Box<dyn Error>> {
    // Convert mnemonic to seed (BIP39)
    let seed = mnemonic.to_seed("");

    // Derive master extended private key from seed (BIP32)
    let master_key: XPrv = XPrv::new(seed)?;

    // BIP44 path: m/44'/coin'/0'/0/0
    // coin_type: 0 for mainnet, 1 for testnet/signet/regtest
    let coin_type = match network {
        Network::Bitcoin => 0,
        Network::Testnet | Network::Signet | Network::Regtest | Network::Testnet4 => 1,
    };

    let path = DerivationPath::from_str(&format!("m/44'/{}'/0'/0/0", coin_type))?;

    // Derive the child key
    let mut derived_key = master_key;
    for child in path {
        derived_key = derived_key.derive_child(child)?;
    }

    // Get the public key
    let public_key = derived_key.public_key();
    let public_key_bytes = public_key.to_bytes();

    // Convert to bitcoin CompressedPublicKey
    let compressed_pubkey = CompressedPublicKey::from_slice(&public_key_bytes)?;

    // Create P2PKH address
    let address = Address::p2pkh(compressed_pubkey, network);

    Ok(address.to_string())
}
