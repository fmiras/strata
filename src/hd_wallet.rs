use bip39::Mnemonic;
use bip32::{Prefix, XPrv};
use bitcoin::Network;
use std::error::Error;

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
