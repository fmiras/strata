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

/// Transaction output from mempool.space API
#[derive(Debug, Deserialize)]
pub struct TxOutput {
    pub scriptpubkey: String,
    pub scriptpubkey_type: String,
    pub value: u64,
}

/// Transaction response from mempool.space API
#[derive(Debug, Deserialize)]
pub struct Transaction {
    // pub txid: String,
    pub vout: Vec<TxOutput>,
}

fn get_base_url(network: Network) -> Result<&'static str, Box<dyn std::error::Error>> {
    match network {
        Network::Bitcoin => Ok("https://mempool.space/api"),
        Network::Testnet => Ok("https://mempool.space/testnet/api"),
        Network::Signet => Ok("https://mempool.space/signet/api"),
        _ => Err("Unsupported network for mempool API".into()),
    }
}

/// Fetch UTXOs for an address from mempool.space API
pub fn fetch_utxos(address: &str, network: Network) -> Result<Vec<Utxo>, Box<dyn std::error::Error>> {
    let base_url = get_base_url(network)?;
    let url = format!("{}/address/{}/utxo", base_url, address);
    let client = reqwest::blocking::Client::new();
    let response = client.get(&url).send()?;

    if !response.status().is_success() {
        return Err(format!("API error: {}", response.status()).into());
    }

    let utxos: Vec<Utxo> = response.json()?;
    Ok(utxos)
}

/// Fetch transaction details from mempool.space API
pub fn fetch_tx(txid: &str, network: Network) -> Result<Transaction, Box<dyn std::error::Error>> {
    let base_url = get_base_url(network)?;
    let url = format!("{}/tx/{}", base_url, txid);
    let client = reqwest::blocking::Client::new();
    let response = client.get(&url).send()?;

    if !response.status().is_success() {
        return Err(format!("API error: {}", response.status()).into());
    }

    let tx: Transaction = response.json()?;
    Ok(tx)
}

/// Broadcast a raw transaction to the network via mempool.space API
pub fn broadcast_tx(tx_hex: &str, network: Network) -> Result<String, Box<dyn std::error::Error>> {
    let base_url = get_base_url(network)?;
    let url = format!("{}/tx", base_url);
    let client = reqwest::blocking::Client::new();
    let response = client.post(&url)
        .header("Content-Type", "text/plain")
        .body(tx_hex.to_string())
        .send()?;

    if !response.status().is_success() {
        let error_text = response.text()?;
        return Err(format!("Broadcast failed: {}", error_text).into());
    }

    let txid = response.text()?;
    Ok(txid)
}
