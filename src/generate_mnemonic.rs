use bip39::{Mnemonic, Language};
use rand::RngCore;

pub fn generate_mnemonic(word_count: usize) {
    // Determine entropy size based on word count
    let entropy_size = match word_count {
        12 => 16,  // 128 bits = 16 bytes
        24 => 32,  // 256 bits = 32 bytes
        _ => {
            eprintln!("Error: Only 12 or 24 words are supported.");
            return;
        }
    };

    // random vector of bytes
    let mut entropy = vec![0u8; entropy_size];
    rand::thread_rng().fill_bytes(&mut entropy);
    
    let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy)
        .expect("Failed to generate mnemonic");
    
    println!("⚠️  This seed is for testing purposes only. Do not use them for real world use.");
    println!("--------------------------------------------------");
    println!("{}", mnemonic.words().collect::<Vec<_>>().join(" "));
    println!("--------------------------------------------------");
}
