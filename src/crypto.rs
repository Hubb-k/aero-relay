use anyhow::Result;
use tracing::info;
use lazy_static::lazy_static;
use snow::params::NoiseParams;

// Noise handshake parameters (ready for future ZK integration)
lazy_static! {
    static ref PARAMS: NoiseParams = "Noise_XX_25519_ChaChaPoly_BLAKE2s"
        .parse()
        .expect("Invalid Noise params");
}

// Placeholder encrypter – will be used for packet encryption over Noise
pub struct PacketEncrypter;

impl PacketEncrypter {
    /// Prepares packet data for sending.
    /// Currently just copies the data (encryption is WIP).
    pub fn prepare_for_send(data: &[u8]) -> Result<Vec<u8>> {
        info!("Preparing packet for send (encryption WIP) – size: {} bytes", data.len());
        Ok(data.to_vec())
    }
}