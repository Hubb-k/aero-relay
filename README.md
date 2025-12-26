# AeroRelay

WIP IBC relay over QUIC with optional ZK encryption proofs.

**Windows users:** Run in WSL2 (Ubuntu recommended).

## Setup
1. Fill `config.toml` (use `config.toml.example` as template)
2. (Optional) Create `.env` from `.env.example` and set `RELAYER_SIGNER`

## Run
```sh
cargo run

With ZK proofs:
cargo run --features encryption-proof


MIT License. Still WIP — flies, but not yet to the moon ✈️
Technical implementation powered by Grok and Gemini.