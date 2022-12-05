use crate::Message;
use rand::Rng;

impl Message {
    // Tested from live mainnet results
    pub fn payload(&self) -> Vec<u8> {
        let mut rng = rand::thread_rng();
        let mut message = match self {
            Message::BeaconBlock { .. } => {
                let bytes: u32 = rng.gen_range(60_000..80_000);
                vec![0; bytes as usize]
                // Thread is some random values to make the payloads distinct
            }
            Message::AggregateAndProofAttestation { .. } => {
                let bytes: u32 = rng.gen_range(500..550);
                vec![0; bytes as usize]
            }
            Message::Attestation { .. } => {
                let bytes: u32 = rng.gen_range(200..310);
                vec![0; bytes as usize]
            }
            Message::SignedContributionAndProof { .. } => {
                let bytes: u32 = rng.gen_range(410..430);
                vec![0; bytes as usize]
            }
            Message::SyncCommitteeMessage { .. } => {
                let bytes: u32 = rng.gen_range(190..210);
                vec![0; bytes as usize]
            }
        };

        // Ranomize the first 8 bits to make sure the message is unique.
        for index in 0..8 {
            message[index] = rand::random();
        }

        message
    }
}
