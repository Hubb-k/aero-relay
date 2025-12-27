<img src="asset/banner.jpg" alt="AeroRelay Banner" width="100%"/>

# AeroRelay ✈️🛡️

**Experimental ZK-focused IBC relayer on Rust**  
WIP — flies on mainnet, but not yet to the moon.

AeroRelay introduces a new approach to cross-chain interoperability in the Cosmos ecosystem. Instead of traditional burn/mint bridges or simple relayers, it implements the **"Jurisdiction Switch"** concept: a token does not merely move — it mathematically changes jurisdiction with ZK privacy and security guarantees.

## Key Features

- **QUIC transport** instead of TCP — faster, more resilient to lag (up to 300 ms), reliable in unstable networks.
- **ZK proofs (Groth16)** — compact (~160 bytes), generated in 4–8 seconds on consumer hardware (6 GB RAM).
- **Proof of encryption/commitment** — proves honest packet handling without revealing sender, receiver, or amount.
- **Mainnet-tested** — captures and processes real traffic on Cosmos Hub ↔ Osmosis using public nodes.
- **Lightweight and efficient** — runs on consumer hardware, no Hermes dependencies.
**Requirements:** Rust (stable), Cargo.
## Quic start
1. Clone the repository:
git clone https://github.com/Hubb-k/aero-relay.git
   cd aero-relay
## Setup
1. Fill `config.toml` (use `config.toml.example` as template)
2. (Optional) Create `.env` from `.env.example` and set `RELAYER_SIGNER`

**Windows:** Run in WSL2 (Ubuntu recommended).

## Technical Details

- Fully written in **Rust** (no Go, no Hermes dependencies).
- Block polling from public RPC nodes.
- IBC packet parsing (including complex memos and swaps).
- Groth16 proof generation for encryption/commitment verification.

Project developed in collaboration between human architecture and AI: core logic and design by the author (@Kerim_mX), significant Rust implementation powered by **Grok (xAI)** and Gemini.
## Run
```sh
cargo run
```
## Run with ZK proofs:
```sh
cargo run --features encryption-proof
```
## Roadmap

Full vision available in [VISION.md](VISION.md).

In short:
- **Stage 1 (current)**: PoC with QUIC + basic ZK proof on mainnet.
- **Stage 2**: Selective disclosure, dynamic VKs, ICA/ICQ support.
- **Stage 3**: On-chain verifier, recursive batching, expansion to other networks.
- **Stage 4**: Decentralized relay network with rewards.


## Contributing
[CONTACTS](CONTACTS.md).
Early stage project — issues, PRs, and ideas are welcome!  
Contact: [@Kerim_mX](https://x.com/Kerim_mX) on X.

## License

MIT [License](LICENSE). Still WIP — flies, but not yet to the moon 
Technical implementation powered by Grok and Gemini.
