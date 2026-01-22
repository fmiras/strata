use bitcoin::Network;
use serde::Deserialize;

/// UTXO response from mempool.space API
#[derive(Debug, Deserialize)]
pub struct Utxo {
    pub txid: String,
    pub vout: u32,
    pub value: u64,
    pub status: UtxoStatus,
}

#[derive(Debug, Deserialize)]
pub struct UtxoStatus {
    pub confirmed: bool,
    #[serde(default)]
    pub block_height: Option<u64>,
}

/// Fetch UTXOs for an address from mempool.space API
pub fn fetch_utxos(address: &str, network: Network) -> Result<Vec<Utxo>, Box<dyn std::error::Error>> {
    let base_url = match network {
        Network::Bitcoin => "https://mempool.space/api",
        Network::Testnet => "https://mempool.space/testnet/api",
        Network::Signet => "https://mempool.space/signet/api",
        _ => return Err("Unsupported network for mempool API".into()),
    };

    let url = format!("{}/address/{}/utxo", base_url, address);
    let client = reqwest::blocking::Client::new();
    let response = client.get(&url).send()?;

    if !response.status().is_success() {
        return Err(format!("API error: {}", response.status()).into());
    }

    let utxos: Vec<Utxo> = response.json()?;
    Ok(utxos)
}
