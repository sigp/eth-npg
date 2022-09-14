use lighthouse_network::PubsubMessage;
use types::eth_spec::MainnetEthSpec;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

fn do_stuff(msg: PubsubMessage<MainnetEthSpec>) {
    match msg {
        PubsubMessage::BeaconBlock(_) => todo!(),
        PubsubMessage::AggregateAndProofAttestation(_) => todo!(),
        PubsubMessage::Attestation(_) => todo!(),
        PubsubMessage::VoluntaryExit(_) => todo!(),
        PubsubMessage::ProposerSlashing(_) => todo!(),
        PubsubMessage::AttesterSlashing(_) => todo!(),
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
