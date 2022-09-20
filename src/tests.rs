use futures::StreamExt;

use super::*;

#[tokio::test]
async fn test_expected_message_count_per_slot() {
    // TODO: rand? quickcheck
    let test_slot = Slot::new(8315647704);
    const EXPECTED_BLOCKS: usize = 1;
    const EXPECTED_AGGREGATES: usize = 16 * 64;
    let total_validators = 16 * 64;
    let all_validators = (0..total_validators).into_iter().collect::<HashSet<_>>();
    let g = Generator::<SystemTimeSlotClock, String>::new(
        Slot::new(0),
        Duration::ZERO,
        3,
        Duration::from_secs(2),
        all_validators,
        total_validators,
    );
    let blocks = g.get_msg(test_slot, MsgType::BeaconBlock);
    let aggregators = g.get_msg(test_slot, MsgType::AggregateAndProofAttestation);
    assert_eq!(blocks.len(), EXPECTED_BLOCKS);
    assert_eq!(aggregators.len(), EXPECTED_AGGREGATES);
}

#[tokio::test]
async fn test_timings() {}
