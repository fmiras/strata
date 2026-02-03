use bip39::Mnemonic;
use keyring::Entry;
use rand::RngCore;
use std::error::Error;

const SERVICE_NAME: &str = "strata-wallet";
const ACCOUNT_NAME: &str = "default";

pub fn save_mnemonic(phrase: &str) -> Result<(), Box<dyn Error>> {
    let entry = Entry::new(SERVICE_NAME, ACCOUNT_NAME)?;
    entry.set_password(phrase)?;
    println!("✅ Mnemonic (BIP39) saved in keychain.");
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

    Ok(Mnemonic::from_entropy_in(bip39::Language::English, &entropy)?)
}

pub fn generate_and_save_mnemonic(word_count: usize) -> Result<Mnemonic, Box<dyn Error>> {
    let mnemonic = generate_mnemonic(word_count)?;
    save_mnemonic(&mnemonic.words().collect::<Vec<_>>().join(" "))?;
    Ok(mnemonic)
}

/// Retrieves the mnemonic. 
/// Returns Result<Option<Mnemonic>>: 
/// - Ok(Some) = Wallet found.
/// - Ok(None) = No wallet exists (not an error, just empty).
/// - Err = Keychain is locked or OS error.
pub fn load_mnemonic() -> Result<Option<Mnemonic>, Box<dyn Error>> {
    let entry = Entry::new(SERVICE_NAME, ACCOUNT_NAME)?;

    match entry.get_password() {
        Ok(password) => {
            if password.is_empty() {
                Ok(None)
            } else {
                let mnemonic = Mnemonic::parse_in(bip39::Language::English, &password)?;
                Ok(Some(mnemonic))
            }
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(Box::new(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_mnemonic_12_words() {
        let result = generate_mnemonic(12);
        assert!(result.is_ok(), "Should generate 12-word mnemonic");
        let mnemonic = result.unwrap();
        let words: Vec<&str> = mnemonic.words().collect();
        assert_eq!(words.len(), 12, "Should have exactly 12 words");
    }

    #[test]
    fn test_generate_mnemonic_24_words() {
        let result = generate_mnemonic(24);
        assert!(result.is_ok(), "Should generate 24-word mnemonic");
        let mnemonic = result.unwrap();
        let words: Vec<&str> = mnemonic.words().collect();
        assert_eq!(words.len(), 24, "Should have exactly 24 words");
    }

    #[test]
    fn test_generate_mnemonic_invalid_word_count() {
        let result = generate_mnemonic(15);
        assert!(result.is_err(), "Should reject 15-word mnemonic");
    }

    #[test]
    fn test_generate_mnemonic_is_random() {
        let mnemonic1 = generate_mnemonic(12).unwrap();
        let mnemonic2 = generate_mnemonic(12).unwrap();

        let words1: Vec<&str> = mnemonic1.words().collect();
        let words2: Vec<&str> = mnemonic2.words().collect();

        assert_ne!(words1, words2, "Two generated mnemonics should be different");
    }

    #[test]
    fn test_generate_mnemonic_valid_bip39_words() {
        let mnemonic = generate_mnemonic(12).unwrap();
        let words: Vec<&str> = mnemonic.words().collect();

        let phrase = words.join(" ");
        let parsed = Mnemonic::parse_in(bip39::Language::English, &phrase);
        assert!(parsed.is_ok(), "Generated mnemonic should be valid BIP39");
    }
}
