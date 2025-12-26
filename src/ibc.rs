use anyhow::{Context, Result};
use tendermint::block::Height;
use tendermint_rpc::{Client, HttpClient};
use tokio::time::{sleep, Duration, Instant};
use tracing::{debug, error, info, warn};
use hex;
use serde_json::Value;

use ibc_proto::ibc::applications::transfer::v2::FungibleTokenPacketData as ProtoFungibleTokenPacketData;
use ibc_proto::ibc::core::channel::v1::{MsgRecvPacket, Packet};
use ibc_proto::ibc::core::client::v1::Height as IbcHeight;

#[derive(Debug)]
pub struct FungibleTokenPacketData {
    pub amount: String,
    pub denom: String,
    pub sender: String,
    pub receiver: String,
}

#[derive(Debug)]
pub struct ParsedPacket {
    pub sequence: u64,
    pub src_port: String,
    pub src_channel: String,
    pub dst_port: String,
    pub dst_channel: String,
    pub timeout_height: String,
    pub timeout_timestamp: u64,
    pub data: FungibleTokenPacketData,
}

pub struct IbcPoller {
    client: HttpClient,
    channel_id: String,
    last_height: u64,
}

impl IbcPoller {
    /// Initialize poller for a specific channel
    pub async fn new(rpc_url: &str, channel_id: &str) -> Result<Self> {
        let client = HttpClient::new(rpc_url)
            .context(format!("Failed to connect to RPC: {}", rpc_url))?;

        let info = client.abci_info().await
            .context("Failed to get ABCI info during initialization")?;
        let last_height = info.last_block_height.value();

        info!("Poller initialized: channel {}, starting height {}", channel_id, last_height);

        Ok(Self {
            client,
            channel_id: channel_id.to_string(),
            last_height,
        })
    }

    /// Process a detected IBC packet (forms MsgRecvPacket and optional ZK proof)
    async fn relay_packet(&self, parsed: &ParsedPacket, packet_data_hex: &str) -> Result<()> {
        let packet_start = Instant::now();

        info!("Forming MsgRecvPacket for sequence {}", parsed.sequence);

        let fungible_data = ProtoFungibleTokenPacketData {
            denom: parsed.data.denom.clone(),
            amount: parsed.data.amount.clone(),
            sender: parsed.data.sender.clone(),
            receiver: parsed.data.receiver.clone(),
            memo: "".to_string(),
        };

        let mut data_bytes = Vec::new();
        prost::Message::encode(&fungible_data, &mut data_bytes)
            .context("Failed to encode FungibleTokenPacketData")?;

        let revision_height = parsed.timeout_height
            .split('-')
            .nth(1)
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        let packet = Packet {
            sequence: parsed.sequence,
            source_port: parsed.src_port.clone(),
            source_channel: parsed.src_channel.clone(),
            destination_port: parsed.dst_port.clone(),
            destination_channel: parsed.dst_channel.clone(),
            data: data_bytes,
            timeout_height: if revision_height > 0 {
                Some(IbcHeight {
                    revision_number: 1,
                    revision_height,
                })
            } else {
                None
            },
            timeout_timestamp: parsed.timeout_timestamp,
        };

        let msg = MsgRecvPacket {
            packet: Some(packet),
            proof_commitment: vec![],
            proof_height: Some(IbcHeight {
                revision_number: 0,
                revision_height: self.last_height,
            }),
            signer: std::env::var("RELAYER_SIGNER")
                .unwrap_or_else(|_| "replace_with_your_address".to_string()),
        };

        info!("MsgRecvPacket formed successfully!");
        info!("  Sequence: {}", msg.packet.as_ref().unwrap().sequence);
        info!("  Src: {} / {}", msg.packet.as_ref().unwrap().source_port, msg.packet.as_ref().unwrap().source_channel);
        info!("  Dst: {} / {}", msg.packet.as_ref().unwrap().destination_port, msg.packet.as_ref().unwrap().destination_channel);
        info!("  Amount: {} {}", parsed.data.amount, parsed.data.denom);
        info!("  Signer: {}", msg.signer);

        #[cfg(feature = "encryption-proof")]
        {
            info!("Launching ZK proof generation...");
            let zk_start = Instant::now();

            match crate::generate_packet_proof(packet_data_hex) {
                Ok(proof) => {
                    let zk_time = zk_start.elapsed().as_millis() as f64 / 1000.0;
                    info!("ZK proof generated successfully (size: {} bytes, time: {:.3} sec)", proof.len(), zk_time);
                }
                Err(e) => {
                    error!("ZK proof generation failed: {:?}", e);
                }
            }
        }

        let packet_duration = packet_start.elapsed();
        let packet_secs = packet_duration.as_secs_f64();

        info!("Packet processing metrics (sequence {}):", parsed.sequence);
        info!("   Total time: {:.3} sec", packet_secs);

        Ok(())
    }

    /// Main polling loop – monitors new blocks and processes relevant IBC packets
    pub async fn poll(&mut self) -> Result<()> {
        info!("Polling started for channel {}", self.channel_id);

        loop {
            let current_height = match self.client.abci_info().await {
                Ok(info) => info.last_block_height.value(),
                Err(e) => {
                    warn!("Failed to get current block height: {}. Retrying in 10 sec...", e);
                    sleep(Duration::from_secs(10)).await;
                    continue;
                }
            };

            while self.last_height < current_height {
                self.last_height += 1;
                let height = Height::try_from(self.last_height)
                    .context("Failed to convert height to tendermint::Height")?;

                debug!("Processing block {}", self.last_height);

                match self.client.block_results(height).await {
                    Ok(results) => {
                        if let Some(txs_results) = results.txs_results {
                            for tx_res in txs_results {
                                for event in tx_res.events {
                                    let is_relevant = (event.kind == "send_packet" || event.kind == "write_acknowledgement")
                                        && event.attributes.iter().any(|a| {
                                            let key = a.key_str().unwrap_or("");
                                            let value = a.value_str().unwrap_or("");
                                            (key == "packet_src_channel" || key == "packet_dst_channel")
                                                && value == self.channel_id
                                        });

                                    if is_relevant {
                                        info!("[Block {}] IBC PACKET DETECTED!", self.last_height);

                                        let mut sequence = 0u64;
                                        let mut src_port = String::new();
                                        let mut src_channel = String::new();
                                        let mut dst_port = String::new();
                                        let mut dst_channel = String::new();
                                        let mut timeout_height = String::new();
                                        let mut timeout_timestamp = 0u64;
                                        let mut packet_data_hex = String::new();

                                        for attr in &event.attributes {
                                            let key = attr.key_str().unwrap_or("");
                                            let value = attr.value_str().unwrap_or("");

                                            match key {
                                                "packet_sequence" => sequence = value.parse().unwrap_or(0),
                                                "packet_src_port" => src_port = value.to_string(),
                                                "packet_src_channel" => src_channel = value.to_string(),
                                                "packet_dst_port" => dst_port = value.to_string(),
                                                "packet_dst_channel" => dst_channel = value.to_string(),
                                                "packet_timeout_height" => timeout_height = value.to_string(),
                                                "packet_timeout_timestamp" => timeout_timestamp = value.parse().unwrap_or(0),
                                                "packet_data_hex" => packet_data_hex = value.to_string(),
                                                _ => {}
                                            }

                                            info!("   {} = {}", key, value);
                                        }

                                        if !packet_data_hex.is_empty() {
                                            match hex::decode(&packet_data_hex) {
                                                Ok(bytes) => {
                                                    let packet_str = String::from_utf8_lossy(&bytes);
                                                    match serde_json::from_str::<Value>(&packet_str) {
                                                        Ok(v) => {
                                                            info!("   Packet parsing (human-readable):");
                                                            info!("     Amount: {}", v["amount"].as_str().unwrap_or("0"));
                                                            info!("     Denom: {}", v["denom"].as_str().unwrap_or(""));
                                                            info!("     Sender: {}", v["sender"].as_str().unwrap_or(""));
                                                            info!("     Receiver: {}", v["receiver"].as_str().unwrap_or(""));

                                                            let parsed = ParsedPacket {
                                                                sequence,
                                                                src_port,
                                                                src_channel,
                                                                dst_port,
                                                                dst_channel,
                                                                timeout_height,
                                                                timeout_timestamp,
                                                                data: FungibleTokenPacketData {
                                                                    amount: v["amount"].as_str().unwrap_or("0").to_string(),
                                                                    denom: v["denom"].as_str().unwrap_or("").to_string(),
                                                                    sender: v["sender"].as_str().unwrap_or("").to_string(),
                                                                    receiver: v["receiver"].as_str().unwrap_or("").to_string(),
                                                                },
                                                            };

                                                            info!("   Full packet structure: {:?}", parsed);

                                                            if let Err(e) = self.relay_packet(&parsed, &packet_data_hex).await {
                                                                error!("Failed to relay packet: {:?}", e);
                                                            }
                                                        }
                                                        Err(e) => warn!("Failed to parse packet JSON: {}", e),
                                                    }
                                                }
                                                Err(e) => warn!("Failed to decode packet hex: {}", e),
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => debug!("Failed to get block results for height {}: {}", self.last_height, e),
                }

                sleep(Duration::from_millis(200)).await;
            }

            sleep(Duration::from_secs(6)).await;
        }
    }
}