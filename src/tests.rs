use futures::StreamExt;

use super::*;

#[tokio::test]
async fn test_expected_message_count_per_slot() {
    // TODO: rand? quickcheck
    let test_slot = Slot::new(8315647704);
    const SLOTS_PER_EPOCH: u64 = 3;
    const TOTAL_VALIDATORS: u64 = 16 * 64 + 38;
    const EXPECTED_BLOCKS: usize = 1;
    const EXPECTED_AGGREGATES: usize = 16 * 64;
    const EXPECTED_ATTESTATIONS_PER_SLOT: usize = (TOTAL_VALIDATORS / SLOTS_PER_EPOCH) as usize;
    let all_validators = (0..TOTAL_VALIDATORS).into_iter().collect::<HashSet<_>>();
    let g = Generator::<SystemTimeSlotClock, String>::new(
        Slot::new(0),
        Duration::ZERO,
        SLOTS_PER_EPOCH,
        Duration::from_secs(2),
        64,
        all_validators,
        TOTAL_VALIDATORS,
    );
    let blocks = g.get_msg(test_slot, MsgType::BeaconBlock);
    let aggregators = g.get_msg(test_slot, MsgType::AggregateAndProofAttestation);
    let epoch = test_slot.epoch(SLOTS_PER_EPOCH);
    let slot_iter = epoch.slot_iter(SLOTS_PER_EPOCH);

    let mut total_attestations = 0;
    for current_slot in slot_iter {
        let slot_attesters = g.get_msg(current_slot, MsgType::Attestation);
        assert!(
            slot_attesters
                .len()
                .abs_diff(EXPECTED_ATTESTATIONS_PER_SLOT)
                <= 1
        ); // tolerance
        total_attestations += slot_attesters.len();
    }

    assert_eq!(blocks.len(), EXPECTED_BLOCKS);
    assert_eq!(aggregators.len(), EXPECTED_AGGREGATES);
    assert_eq!(total_attestations, TOTAL_VALIDATORS as usize);
}

#[tokio::test]
async fn test_timings() {}
