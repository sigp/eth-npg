use super::*;
use quickcheck_macros::quickcheck;

#[quickcheck]
fn test_expected_message_counts(
    test_slot: u64,
    slots_per_epoch: u64,
    total_validators: u64,
    subnets: u64,
) -> quickcheck::TestResult {
    let total_aggregators = match super::TARGET_AGGREGATORS.checked_mul(subnets) {
        Some(count) => count,
        None => {
            // if this is too big just skip the test
            return quickcheck::TestResult::discard();
        }
    };
    if slots_per_epoch == 0
        || subnets == 0
        || total_validators == 0
        || total_aggregators > total_validators
        || total_validators > u32::MAX as u64
    {
        // Need at least one validator in the whole network
        return quickcheck::TestResult::discard();
    }

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            test_expected_message_counts_fn(test_slot, slots_per_epoch, total_validators, subnets)
        });

    quickcheck::TestResult::passed()
}

fn test_expected_message_counts_fn(
    test_slot: u64,
    slots_per_epoch: u64,
    total_validators: u64,
    subnets: u64,
) {
    let test_slot = Slot::new(test_slot);

    // Expected number of messages
    let expected_blocks: usize = 1;
    let expected_aggregates: usize = (super::TARGET_AGGREGATORS * subnets) as usize;
    let expected_attestations_per_slot: usize = (total_validators / slots_per_epoch) as usize;

    // Test on a centralized network
    let all_validators = (0..total_validators).into_iter().collect::<HashSet<_>>();

    let g = Generator::<SystemTimeSlotClock, String>::new(
        Slot::new(0),
        Duration::ZERO,
        slots_per_epoch,
        Duration::from_secs(2),
        subnets,
        all_validators,
        total_validators,
    );

    // Test the number of expected blocks.
    let blocks = g.get_msg(test_slot, MsgType::BeaconBlock);
    assert_eq!(blocks.len(), expected_blocks);

    // Test the number of expected aggregators per slot.
    let aggregators = g.get_msg(test_slot, MsgType::AggregateAndProofAttestation);
    assert_eq!(aggregators.len(), expected_aggregates);

    // Test the number of attestations per slot and epoch.
    let epoch = test_slot.epoch(slots_per_epoch);
    let slot_iter = epoch.slot_iter(slots_per_epoch);

    let mut total_attestations = 0;
    for current_slot in slot_iter {
        let slot_attesters = g.get_msg(current_slot, MsgType::Attestation);
        assert!(
            slot_attesters
                .len()
                .abs_diff(expected_attestations_per_slot)
                <= 1
        ); // tolerance
        total_attestations += slot_attesters.len();
    }

    assert_eq!(total_attestations, total_validators as usize);
}

#[tokio::test]
async fn test_timings() {}
