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

#[cfg(test)]
mod tests {
    use super::*;

    // Well-known BIP39 test mnemonic
    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    fn get_test_mnemonic() -> Mnemonic {
        Mnemonic::parse_in(bip39::Language::English, TEST_MNEMONIC).unwrap()
    }

    #[test]
    fn test_address_type_default() {
        let addr_type = AddressType::default();
        assert_eq!(addr_type, AddressType::P2wpkh);
    }

    #[test]
    fn test_derive_bip44_account_xpub_mainnet() {
        let mnemonic = get_test_mnemonic();
        let result = derive_bip44_account_xpub(&mnemonic, Network::Bitcoin);
        assert!(result.is_ok());
        let xpub = result.unwrap();
        assert_eq!(xpub, "xpub6BosfCnifzxcFwrSzQiqu2DBVTshkCXacvNsWGYJVVhhawA7d4R5WSWGFNbi8Aw6ZRc1brxMyWMzG3DSSSSoekkudhUd9yLb6qx39T9nMdj");
    }

    #[test]
    fn test_derive_bip44_account_xpub_signet() {
        let mnemonic = get_test_mnemonic();
        let result = derive_bip44_account_xpub(&mnemonic, Network::Signet);
        assert!(result.is_ok());
        let xpub = result.unwrap();
        assert_eq!(xpub, "tpubDC5FSnBiZDMmhiuCmWAYsLwgLYrrT9rAqvTySfuCCrgsWz8wxMXUS9Tb9iVMvcRbvFcAHGkMD5Kx8koh4GquNGNTfohfk7pgjhaPCdXpoba");
    }

    #[test]
    fn test_derive_bip44_account_xpub_deterministic() {
        let mnemonic = get_test_mnemonic();
        let xpub1 = derive_bip44_account_xpub(&mnemonic, Network::Bitcoin).unwrap();
        let xpub2 = derive_bip44_account_xpub(&mnemonic, Network::Bitcoin).unwrap();
        assert_eq!(xpub1, xpub2, "Same mnemonic should produce same xpub");
    }

    #[test]
    fn test_derive_private_key_mainnet() {
        let mnemonic = get_test_mnemonic();
        let result = derive_private_key(&mnemonic, Network::Bitcoin, 0);
        assert!(result.is_ok());
        let secret_key = result.unwrap();
        assert_eq!(secret_key.secret_bytes().len(), 32);
    }

    #[test]
    fn test_derive_private_key_deterministic() {
        let mnemonic = get_test_mnemonic();
        let key1 = derive_private_key(&mnemonic, Network::Bitcoin, 0).unwrap();
        let key2 = derive_private_key(&mnemonic, Network::Bitcoin, 0).unwrap();
        assert_eq!(key1.secret_bytes(), key2.secret_bytes(), "Same derivation should produce same key");
    }

    #[test]
    fn test_derive_private_key_different_indices() {
        let mnemonic = get_test_mnemonic();
        let key0 = derive_private_key(&mnemonic, Network::Bitcoin, 0).unwrap();
        let key1 = derive_private_key(&mnemonic, Network::Bitcoin, 1).unwrap();
        assert_ne!(key0.secret_bytes(), key1.secret_bytes(), "Different indices should produce different keys");
    }

    #[test]
    fn test_derive_public_key() {
        let mnemonic = get_test_mnemonic();
        let result = derive_public_key(&mnemonic, Network::Bitcoin, 0);
        assert!(result.is_ok());
        let pubkey = result.unwrap();
        // Compressed public key should be 33 bytes
        assert_eq!(pubkey.to_bytes().len(), 33);
        // First byte should be 0x02 or 0x03 for compressed pubkey
        let first_byte = pubkey.to_bytes()[0];
        assert!(first_byte == 0x02 || first_byte == 0x03);
    }

    #[test]
    fn test_derive_public_key_deterministic() {
        let mnemonic = get_test_mnemonic();
        let pubkey1 = derive_public_key(&mnemonic, Network::Bitcoin, 0).unwrap();
        let pubkey2 = derive_public_key(&mnemonic, Network::Bitcoin, 0).unwrap();
        assert_eq!(pubkey1.to_bytes(), pubkey2.to_bytes());
    }

    #[test]
    fn test_derive_address_p2pk() {
        let mnemonic = get_test_mnemonic();
        let xpub = derive_bip44_account_xpub(&mnemonic, Network::Bitcoin).unwrap();
        let address = derive_address_from_xpub(&xpub, Network::Bitcoin, 0, AddressType::P2pk).unwrap();
        assert!(hex::decode(&address).is_ok(), "P2PK should be valid hex");
        assert_eq!(address, "03aaeb52dd7494c361049de67cc680e83ebcbbbdbeb13637d92cd845f70308af5e");
    }

    #[test]
    fn test_derive_address_p2pkh() {
        let mnemonic = get_test_mnemonic();
        let xpub = derive_bip44_account_xpub(&mnemonic, Network::Bitcoin).unwrap();
        let address = derive_address_from_xpub(&xpub, Network::Bitcoin, 0, AddressType::P2pkh).unwrap();
        assert_eq!(address,"1LqBGSKuX5yYUonjxT5qGfpUsXKYYWeabA");
    }

    #[test]
    fn test_derive_address_p2wpkh() {
        let mnemonic = get_test_mnemonic();
        let xpub = derive_bip44_account_xpub(&mnemonic, Network::Bitcoin).unwrap();
        let address = derive_address_from_xpub(&xpub, Network::Bitcoin, 0, AddressType::P2wpkh).unwrap();
        assert_eq!(address, "bc1qmxrw6qdh5g3ztfcwm0et5l8mvws4eva24kmp8m")
    }

    #[test]
    fn test_derive_address_p2tr() {
        let mnemonic = get_test_mnemonic();
        let xpub = derive_bip44_account_xpub(&mnemonic, Network::Bitcoin).unwrap();
        let address = derive_address_from_xpub(&xpub, Network::Bitcoin, 0, AddressType::P2tr).unwrap();
        assert_eq!(address, "bc1plguuppjuw5uk2rpyjnnzvwsuvy5ctswns9fsvhrvn4qt04ns4nmscf9eqf");
    }

    #[test]
    fn test_derive_address_deterministic() {
        let mnemonic = get_test_mnemonic();
        let xpub = derive_bip44_account_xpub(&mnemonic, Network::Bitcoin).unwrap();
        let addr1 = derive_address_from_xpub(&xpub, Network::Bitcoin, 0, AddressType::P2wpkh).unwrap();
        let addr2 = derive_address_from_xpub(&xpub, Network::Bitcoin, 0, AddressType::P2wpkh).unwrap();
        assert_eq!(addr1, addr2, "Same derivation should produce same address");
    }

    #[test]
    fn test_derive_address_different_indices() {
        let mnemonic = get_test_mnemonic();
        let xpub = derive_bip44_account_xpub(&mnemonic, Network::Bitcoin).unwrap();
        let addr0 = derive_address_from_xpub(&xpub, Network::Bitcoin, 0, AddressType::P2wpkh).unwrap();
        let addr1 = derive_address_from_xpub(&xpub, Network::Bitcoin, 1, AddressType::P2wpkh).unwrap();
        assert_ne!(addr0, addr1, "Different indices should produce different addresses");
    }

    #[test]
    fn test_derive_address_invalid_xpub() {
        let result = derive_address_from_xpub("invalid_xpub", Network::Bitcoin, 0, AddressType::P2wpkh);
        assert!(result.is_err(), "Invalid xpub should fail");
    }
}
