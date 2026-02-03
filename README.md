# Strata Wallet 🛰️
**The Archaeological Bitcoin Wallet**

`Strata` is a CLI-based Bitcoin wallet built in Rust for educational purposes. Unlike production wallets that hide the complexity of the protocol, Strata is designed to expose the raw machinery of Bitcoin. 

This project is an "archaeological dig" through Bitcoin's script history—traversing from the **Satoshi-era P2PK** bedrock to the modern **Taproot** surface.

## 🎯 Project Goals
* **Manual Scripting:** Construct every major Bitcoin locking (`scriptPubKey`) and unlocking (`scriptSig`/`witness`) script manually.
* **Granular Control:** No automated coin selection. You choose the UTXOs, you calculate the weights, and you set the fees.
* **Full Compatibility:** Support for P2PK, P2PKH, P2SH, P2WPKH, P2WSH, and P2TR.
* **Protocol Mastery:** Deep dive into Sighash algorithms, Bech32m encoding, and the Bitcoin Stack Machine.

## 🏗️ Technical Architecture



| Component | Responsibility | Tools |
| :--- | :--- | :--- |
| **Identity** | BIP39 Mnemonics & BIP32 Key Trees | `bip39`, `rust-bitcoin` |
| **Script Lab** | Logic for P2PK, P2PKH, SegWit, Taproot | `rust-bitcoin` (Builder) |
| **Observer** | Fetching UTXOs and network state | `esplora-client` (Signet) |
| **Craftsman** | Manual Tx assembly and manual signing | `rust-bitcoin` (Transaction) |

## Testing

You can use real Bitcoins at any time to test the different functionalities of the wallet, but it is suggested to use testnet as the wallet was not deeply tested nor audited. For using Signet testnet you can do the following:
- Get some coins by using [Signet BTC Faucet](https://signet.dcorral.com)
    - Make sure to send this to a generated address with network signet flag `strata receive --network signet`
- For receiving some classic P2PK (unsupported by modern wallets) you can use this [P2PK Playground](https://supertestnet.github.io/p2pk_playground)

---

## TODO

### Phase 1: The Seed (Foundation) 
- [x] Initialize Rust project.
- [x] Implement BIP-39: Generate Mnemonic and 512-bit Seed.
- [x] Implement BIP-32: Master Xpriv derivation and child key derivation.

### Phase 2: The Address Book (Reception)
- [x] Create an "Address Generator" command to derive:
    - [x] **P2PK** (Raw Pubkey Hex - No standard address)
    - [x] **P2PKH** (Legacy - `1...` or `m/n...` for Testnet)
    - [x] **P2WPKH** (Native SegWit - `bc1q...` or `tb1q...`)
    - [x] **P2TR** (Taproot - `bc1p...` or `tb1p...`)

### Phase 3: The Observer (Blockchain Sync)
- [x] Integrate an Esplora client (mempool.space API).
- [x] Create a `utxo ls` command that scans your derived addresses.
- [x] Display UTXO data: TXID, Vout, Amount, and ScriptType.
- [ ] Verify blocks headers as a true SPV client

### Phase 4: The Script Factory (Spending - Legacy & SegWit)
- [ ] Implement manual Transaction Building:
    - [ ] **Input Selection:** User manually selects inputs.
    - [ ] **Sighash Generation:** Manually hash the transaction data.
    - [ ] **The Signing Logic:** Manually sign and attach to `scriptSig` or `witness`.
- [ ] Spend a **P2PK** output (The Satoshi Test).
- [ ] Spend a **P2PKH** output (The Legacy Test).
- [ ] Spend a **P2WPKH** output (The SegWit Test).

### Phase 5: The Advanced Lab (P2SH & Multi-Sig)
- [ ] Implement **P2SH (2-of-3 Multisig)**.
- [ ] Implement **P2WSH** (SegWit Multisig).
- [ ] Create a "Multisig Coordinator" flow to collect signatures.

### Phase 6: The Modern Era (Taproot)
- [ ] Implement **Taproot Key-path spends** (Schnorr signatures).
- [ ] Implement **Taproot Script-path spends** (Merkle Trees/MAST).

---

## 🛠️ Usage
```bash
# Generate a new master seed
strata generate --words 12

# Derive addresses for receiving BTC
strata receive --type p2wpkh

# List available UTXOs across all scripts
strata utxo ls

# Manually craft a spend
strata send --input <TXID:VOUT> --to <ADDR> --fee <SATS_vB>
```

## Misc

- [First ever transaction](https://mempool.space/tx/100a20405407d7dccae6a33af18700659f9b8b697c65371047b04f2d5135c664?mode=details) made with this wallet
