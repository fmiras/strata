use bip32::{ChildNumber, DerivationPath, Prefix, XPrv, XPub};
use bip39::Mnemonic;
use bitcoin::address::Address;
use bitcoin::key::CompressedPublicKey;
use bitcoin::Network;
use std::error::Error;
use std::str::FromStr;

/// Derives the BIP44 account-level extended public key from a mnemonic.
/// Path: m/44'/coin'/0' where coin is 0 for mainnet, 1 for testnet.
/// This xpub can be used to derive receive addresses without the private key.
pub fn derive_bip44_account_xpub(mnemonic: &Mnemonic, network: Network) -> Result<String, Box<dyn Error>> {
    let seed = mnemonic.to_seed("");
    let master_key: XPrv = XPrv::new(seed)?;

    let coin_type = match network {
        Network::Bitcoin => 0,
        Network::Testnet | Network::Signet | Network::Regtest | Network::Testnet4 => 1,
    };

    // BIP44 account path: m/44'/coin'/0'
    let path = DerivationPath::from_str(&format!("m/44'/{}'/0'", coin_type))?;

    let mut derived_key = master_key;
    for child in path {
        derived_key = derived_key.derive_child(child)?;
    }

    let xpub = derived_key.public_key();
    let prefix = match network {
        Network::Bitcoin => Prefix::XPUB,
        Network::Testnet | Network::Signet | Network::Regtest | Network::Testnet4 => Prefix::TPUB,
    };

    Ok(xpub.to_string(prefix))
}

/// Derives a legacy P2PKH address from a BIP44 account xpub.
/// Derives path /0/index (external chain, address at index) from the account xpub.
pub fn derive_address_from_xpub(xpub_str: &str, network: Network, index: u32) -> Result<String, Box<dyn Error>> {
    let xpub = xpub_str.parse::<XPub>()?;

    // Derive /0/index (external chain, address at index)
    let external_chain = xpub.derive_child(ChildNumber::new(0, false)?)?;
    let address_key = external_chain.derive_child(ChildNumber::new(index, false)?)?;

    let public_key_bytes = address_key.to_bytes();
    // if we convert to Hex here we could return classic P2PK (without hashing)
    // Ok(hex::encode(public_key_bytes))

    let compressed_pubkey = CompressedPublicKey::from_slice(&public_key_bytes)?;

    let address = Address::p2pkh(compressed_pubkey, network);
    Ok(address.to_string())
}
