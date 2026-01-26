use bip32::{ChildNumber, DerivationPath, Prefix, XPrv, XPub};
use bip39::Mnemonic;
use bitcoin::address::Address;
use bitcoin::key::CompressedPublicKey;
use bitcoin::secp256k1::{Secp256k1, SecretKey, XOnlyPublicKey};
use bitcoin::Network;
use std::error::Error;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum AddressType {
    #[default]
    P2wpkh,
    P2pkh,
    P2pk,
    P2tr,
}

/// Derives the BIP44 account-level extended public key from a mnemonic.
/// Path: m/44'/coin'/0' where coin is 0 for mainnet, 1 for testnet.
/// This xpub can be used to derive receive addresses without the private key.
pub fn derive_bip44_account_xpub(mnemonic: &Mnemonic, network: Network) -> Result<String, Box<dyn Error>> {
    let seed = mnemonic.to_seed("");
    let master_key: XPrv = XPrv::new(seed)?;

    let coin_type = match network {
        Network::Bitcoin => 0,
        Network::Testnet | Network::Signet | Network::Testnet4 => 1,
        _ => return Err("Unsupported network".into()),
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
        Network::Testnet | Network::Signet | Network::Testnet4 => Prefix::TPUB,
        _ => return Err("Unsupported network".into()),
    };

    Ok(xpub.to_string(prefix))
}

/// Derives a private key from a mnemonic for a specific address index.
/// Path: m/44'/coin'/0'/0/index
pub fn derive_private_key(
    mnemonic: &Mnemonic,
    network: Network,
    index: u32,
) -> Result<SecretKey, Box<dyn Error>> {
    let seed = mnemonic.to_seed("");
    let master_key: XPrv = XPrv::new(seed)?;

    let coin_type = match network {
        Network::Bitcoin => 0,
        Network::Testnet | Network::Signet | Network::Testnet4 => 1,
        _ => return Err("Unsupported network".into()),
    };

    // BIP44 full path: m/44'/coin'/0'/0/index
    let path = DerivationPath::from_str(&format!("m/44'/{}'/0'/0/{}", coin_type, index))?;

    let mut derived_key = master_key;
    for child in path {
        derived_key = derived_key.derive_child(child)?;
    }

    let secret_key = SecretKey::from_slice(&derived_key.to_bytes())?;
    Ok(secret_key)
}

/// Derives the public key for a specific address index from a mnemonic.
pub fn derive_public_key(
    mnemonic: &Mnemonic,
    network: Network,
    index: u32,
) -> Result<CompressedPublicKey, Box<dyn Error>> {
    let secret_key = derive_private_key(mnemonic, network, index)?;
    let secp = Secp256k1::new();
    let public_key = bitcoin::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
    let compressed = CompressedPublicKey(public_key);
    Ok(compressed)
}

/// Derives an address from a BIP44 account xpub.
/// Derives path /0/index (external chain, address at index) from the account xpub.
/// For P2PKH: returns the hashed address (e.g., 1ABC...)
/// For P2PK: returns the raw compressed public key in hex
pub fn derive_address_from_xpub(
    xpub_str: &str,
    network: Network,
    index: u32,
    address_type: AddressType,
) -> Result<String, Box<dyn Error>> {
    let xpub = xpub_str.parse::<XPub>()?;

    // Derive /0/index (external chain, address at index)
    let external_chain = xpub.derive_child(ChildNumber::new(0, false)?)?;
    let address_key = external_chain.derive_child(ChildNumber::new(index, false)?)?;

    let public_key_bytes = address_key.to_bytes();

    match address_type {
        AddressType::P2pk => {
            // Return raw compressed public key in hex
            Ok(hex::encode(public_key_bytes))
        }
        AddressType::P2pkh => {
            // Return hashed P2PKH address
            let compressed_pubkey = CompressedPublicKey::from_slice(&public_key_bytes)?;
            let address = Address::p2pkh(compressed_pubkey, network);
            Ok(address.to_string())
        }
        AddressType::P2wpkh => {
            // Return native SegWit P2WPKH address (bc1q...)
            let compressed_pubkey = CompressedPublicKey::from_slice(&public_key_bytes)?;
            let address = Address::p2wpkh(&compressed_pubkey, network);
            Ok(address.to_string())
        }
        AddressType::P2tr => {
            // Return Taproot P2TR address (bc1p...)
            let secp = Secp256k1::verification_only();
            let x_only_pubkey = XOnlyPublicKey::from_slice(&public_key_bytes[1..])?;
            let address = Address::p2tr(&secp, x_only_pubkey, None, network);
            Ok(address.to_string())
        }
    }
}
