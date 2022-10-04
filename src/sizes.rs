use crate::Message;
use rand::Rng;

impl Message {
    // Is this somewhat believable?
    pub fn payload(&self) -> Vec<u8> {
        let mut rng = rand::thread_rng();
        match self {
            Message::BeaconBlock { .. } => {
                let bytes: u32 = rng.gen_range(60_000..80_000);
                vec![0; bytes as usize]
            }
            Message::AggregateAndProofAttestation { .. } => {
                let bytes: u32 = rng.gen_range(80_000..135_000);
                vec![0; bytes as usize]
            }
            Message::Attestation { .. } => {
                let bytes: u32 = rng.gen_range(30_000..55_000);
                vec![0; bytes as usize]
            }
            Message::SignedContributionAndProof { .. } => {
                let bytes: u32 = rng.gen_range(3_000..8_000);
                vec![0; bytes as usize]
            }
            Message::SyncCommitteeMessage { .. } => {
                let bytes: u32 = rng.gen_range(8_000..20_000);
                vec![0; bytes as usize]
            }
        }
    }
}
