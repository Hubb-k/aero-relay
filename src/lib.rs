pub mod config;
pub mod ibc;
pub mod transport;
pub mod relay;
pub mod crypto;

// ZK module – included only when the encryption-proof feature is enabled
#[cfg(feature = "encryption-proof")]
pub mod zk;

pub use config::Config;
pub use ibc::IbcPoller;

// Export ZK proof generation only when the feature is enabled
#[cfg(feature = "encryption-proof")]
pub use zk::generate_packet_proof;

// Stub when feature is disabled (allows code using generate_packet_proof to compile)
#[cfg(not(feature = "encryption-proof"))]
pub fn generate_packet_proof(_packet_data_hex: &str) -> anyhow::Result<Vec<u8>> {
    Ok(vec![]) // empty proof – just for compilation
}