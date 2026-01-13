use bip39::{Mnemonic, Language};
use keyring::Entry;
use rand::RngCore;
use std::error::Error;

#[cfg(target_os = "macos")]
use security_framework::os::macos::keychain::SecKeychainItem;
use security_framework::os::macos::access_control::{SecAccessControl, SecAccessControlCreateFlags};
use security_framework::base::Error as SecurityError;

const SERVICE_NAME: &str = "strata-wallet";
const ACCOUNT_NAME: &str = "default";

pub fn save_mnemonic(phrase: &str) -> Result<(), Box<dyn Error>> {
    let entry = Entry::new(SERVICE_NAME, ACCOUNT_NAME)?;
    entry.set_password(phrase)?;
    println!("✅ Mnemonic (BIP39) saved in keychain.");
    println!("💡 Note: To enable Touch ID, open Keychain Access, find '{}', double-click it, and enable 'Require Touch ID' in Access Control.", SERVICE_NAME);
    Ok(())
}

pub fn generate_mnemonic(word_count: usize) -> Result<Mnemonic, Box<dyn Error>> {
    let entropy_size = match word_count {
        12 => 16,  // 128 bits = 16 bytes
        24 => 32,  // 256 bits = 32 bytes
        _ => {
            return Err("Error: Only 12 or 24 words are supported.".into());
        }
    };
    let mut entropy = vec![0u8; entropy_size];
    rand::thread_rng().fill_bytes(&mut entropy);

    Ok(Mnemonic::from_entropy_in(Language::English, &entropy)?)
}

pub fn generate_and_save_mnemonic(word_count: usize) -> Result<Mnemonic, Box<dyn Error>> {
    let mnemonic = generate_mnemonic(word_count)?;
    save_mnemonic(&mnemonic.words().collect::<Vec<_>>().join(" "))?;
    Ok(mnemonic)
}

/// Retrieves the mnemonic. 
/// Returns Result<Option<String>>: 
/// - Ok(Some) = Wallet found.
/// - Ok(None) = No wallet exists (not an error, just empty).
/// - Err = Keychain is locked or OS error.
pub fn load_mnemonic() -> Result<Option<String>, Box<dyn Error>> {
    let entry = Entry::new(SERVICE_NAME, ACCOUNT_NAME)?;
    
    match entry.get_password() {
        Ok(phrase) => Ok(Some(phrase)),
        Err(keyring::Error::NoEntry) => Ok(None), 
        Err(e) => Err(Box::new(e)),
    }
}