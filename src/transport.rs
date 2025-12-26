use anyhow::{Context, Result};
use quinn::{Connection, Endpoint, ServerConfig};
use rcgen::generate_simple_self_signed;
use rustls_pki_types::{CertificateDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

#[derive(Debug)]
struct SkipServerVerification;

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        // Required explicit list for rustls 0.23+
        vec![
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
        ]
    }
}

/// Establish a QUIC client connection (skips certificate verification for self-signed certs)
pub async fn establish_connection(dst_addr: &str) -> Result<Connection> {
    let provider = rustls::crypto::aws_lc_rs::default_provider();
    let crypto = rustls::ClientConfig::builder_with_provider(Arc::new(provider))
        .with_safe_default_protocol_versions()?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
        .with_no_client_auth();

    let client_config = quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(crypto)?,
    ));

    let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)
        .context("Failed to create client endpoint")?;
    endpoint.set_default_client_config(client_config);

    let conn = endpoint
        .connect(dst_addr.parse()?, "aero-relay")?
        .await
        .context(format!("Failed to connect via QUIC to {}", dst_addr))?;

    info!("QUIC connection established with {}", dst_addr);
    Ok(conn)
}

/// Send data over an existing QUIC connection (bidirectional stream)
pub async fn send_packet(conn: &Connection, data: Vec<u8>) -> Result<()> {
    let (mut send, _recv) = conn
        .open_bi()
        .await
        .context("Failed to open bidirectional stream")?;

    send.write_all(&data)
        .await
        .context("Failed to write data to QUIC stream")?;

    // In quinn 0.11, finish() returns Result and is not async
    let _ = send.finish();

    info!("Sent {} bytes via QUIC", data.len());
    Ok(())
}

/// Start the QUIC server (self-signed cert, listens indefinitely)
pub async fn start_server(listen_addr: &str) -> Result<()> {
    let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    let cert = generate_simple_self_signed(subject_alt_names)?;

    let cert_der = CertificateDer::from(cert.cert);
    let key_der = PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der());

    let provider = rustls::crypto::aws_lc_rs::default_provider();
    let server_crypto = rustls::ServerConfig::builder_with_provider(Arc::new(provider))
        .with_safe_default_protocol_versions()?
        .with_no_client_auth()
        .with_single_cert(vec![cert_der], key_der.into())
        .context("Failed to create server config")?;

    let server_config = ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(server_crypto)?,
    ));
    let endpoint = Endpoint::server(server_config, listen_addr.parse()?)
        .context("Failed to bind server to address")?;

    info!("QUIC server started on {}", listen_addr);

    while let Some(connecting) = endpoint.accept().await {
        tokio::spawn(async move {
            match connecting.await {
                Ok(new_conn) => {
                    info!("New QUIC connection from {}", new_conn.remote_address());
                    if let Err(e) = handle_connection(new_conn).await {
                        warn!("Error handling connection: {}", e);
                    }
                }
                Err(e) => error!("Error accepting connection: {}", e),
            }
        });
    }
    Ok(())
}

/// Echo received data back to client (simple relay behavior)
async fn handle_connection(conn: Connection) -> Result<()> {
    while let Ok((mut send, mut recv)) = conn.accept_bi().await {
        tokio::spawn(async move {
            let mut buffer = vec![0u8; 64 * 1024];
            match recv.read(&mut buffer).await {
                Ok(Some(len)) => {
                    info!("Received {} bytes via QUIC", len);
                    let _ = send.write_all(&buffer[..len]).await;
                    let _ = send.finish();
                }
                Ok(None) => debug!("Stream closed by client"),
                Err(e) => warn!("Error reading stream: {}", e),
            }
        });
    }
    Ok(())
}