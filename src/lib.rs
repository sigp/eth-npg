use lighthouse_network::PubsubMessage;
use types::eth_spec::MainnetEthSpec;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}


fn dummy_sizes(msg: PubsubMessage<MainnetEthSpec>) -> PubsubMessage {
    match msg {
        PubsubMessage::BeaconBlock(_) => {
            let large_block = BeaconBlockMerge::full(MainnetEthSpec::default_spec());
            let signed_block = SignedBeaconBlock {
                message: large_block,
                signature: Signature::empty()
            };
            PubsubMessage::BeaconBlock(signed_block)
        }
        PubsubMessage::AggregateAndProofAttestation(_) => { 
            let big_attestation = Attestation {
            aggregation_bits: types::BitList::with_capacity(T::MaxValidatorsPerCommittee::to_usize()).unwrap(),
            data: types::AttestationData::default(),
            signature: types::AggregateSignature::empty(),
            };
            let big_aggregate_and_proof = types::AggregateAndProof<MainnetEthSpec> {
                aggregator_index: 1_000_000,
                aggregate: big_attestation,
            }
            let signed = SignedAggregateAndProof {
                    message: big_aggregate_and_proof,
                    signature: Signature::empty()
            };
            PubsubMessage::AggregateAndProof(signed)
        },
        PubsubMessage::Attestation(_) => {
            let big_attestation = Attestation {
            aggregation_bits: types::BitList::with_capacity(T::MaxValidatorsPerCommittee::to_usize()).unwrap(),
            data: types::AttestationData::default(),
            signature: types::AggregateSignature::empty(),
            };
            GossipMessage::Attestation(big_attestation)
        }

        PubsubMessage::VoluntaryExit(_) => {
         let exit = VoluntaryExit {
                    epoch: Epoch::new(1),
                    validator_index: 1,
                };
        PubsubMessage::VoluntaryExit(exit)
        }
        PubsubMessage::ProposerSlashing(_) => { 
        let header = BeaconBlockHeader {
            slot: Slot::new(1),
            proposer_index: 0,
            parent_root: Hash256::zero(),
            state_root: Hash256::zero(),
            body_root: Hash256::zero(),
        };

        let signed_header = SignedBeaconBlockHeader {
            message: header,
            signature: Signature::empty(),
        };
        let proposer_slashing = ProposerSlashing {
            signed_header_1: signed_header.clone(),
            signed_header_2: signed_header,
        };
        PubsubMessage::ProposerSlashing(proposer_slashing)
        }
        PubsubMessage::AttesterSlashing(_) => todo!()
        PubsubMessage::SignedContributionAndProof(_) => todo!(),
        PubsubMessage::SyncCommitteeMessage(_) => todo!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
