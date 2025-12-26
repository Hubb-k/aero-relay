#[cfg(feature = "encryption-proof")]
mod zk_impl {
    use anyhow::{anyhow, Result};
    use blake3::hash;
    use halo2_proofs::halo2curves::bn256::{Bn256, Fr, G1Affine};
    use halo2_proofs::{
        circuit::{Layouter, SimpleFloorPlanner, Value},
        plonk::{Circuit, ConstraintSystem, Error as PlonkError, create_proof, keygen_pk, keygen_vk},
        poly::kzg::{
            commitment::{KZGCommitmentScheme, ParamsKZG},
            multiopen::ProverGWC,
        },
        transcript::{Blake2bWrite, Challenge255, TranscriptWriterBuffer},
    };
    use rand_core::OsRng;
    use tracing::info;

    // Allow dead code since this is a WIP circuit
    #[allow(dead_code)]
    #[derive(Clone)]
    struct PacketCommitmentCircuit {
        preimage: Vec<u8>,
        public_commitment: Value<[u8; 32]>,
    }

    impl Circuit<Fr> for PacketCommitmentCircuit {
        type Config = ();
        type FloorPlanner = SimpleFloorPlanner;

        fn without_witnesses(&self) -> Self {
            Self {
                preimage: Vec::new(),
                public_commitment: Value::unknown(),
            }
        }

        fn configure(_meta: &mut ConstraintSystem<Fr>) -> Self::Config {
            ()
        }

        fn synthesize(
            &self,
            _config: Self::Config,
            _layouter: impl Layouter<Fr>,
        ) -> Result<(), PlonkError> {
            Ok(())
        }
    }

    /// Generates a ZK proof of packet commitment (WIP – currently proves nothing but compiles)
    pub fn generate_packet_proof(packet_data_hex: &str) -> Result<Vec<u8>> {
        let preimage = hex::decode(packet_data_hex)?;
        let commitment: [u8; 32] = hash(&preimage).into();

        let params = ParamsKZG::<Bn256>::setup(11, OsRng);

        let circuit = PacketCommitmentCircuit {
            preimage,
            public_commitment: Value::known(commitment),
        };

        let vk = keygen_vk(&params, &circuit).map_err(|e| anyhow!("VK error: {:?}", e))?;
        let pk = keygen_pk(&params, vk, &circuit).map_err(|e| anyhow!("PK error: {:?}", e))?;

        let mut transcript = Blake2bWrite::<_, G1Affine, Challenge255<_>>::init(vec![]);

        // Empty public instances – no public inputs yet
        let instances: &[Vec<Vec<Fr>>] = &[vec![]];

        create_proof::<KZGCommitmentScheme<Bn256>, ProverGWC<_>, Challenge255<_>, _, _, _>(
            &params,
            &pk,
            &[circuit],
            instances,
            OsRng,
            &mut transcript,
        )
        .map_err(|e| anyhow!("Proof error: {:?}", e))?;

        let proof = transcript.finalize();
        info!("ZK proof generated: {} bytes", proof.len());

        Ok(proof)
    }
}

#[cfg(feature = "encryption-proof")]
pub use zk_impl::generate_packet_proof;

#[cfg(not(feature = "encryption-proof"))]
pub fn generate_packet_proof(_packet_data_hex: &str) -> anyhow::Result<Vec<u8>> {
    Ok(vec![])
}