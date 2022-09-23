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

/// Verifies that the right amount of distinct messages are generated.
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

    let g = Generator::<SystemTimeSlotClock, Message>::new(
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
    let unique_blocks = HashSet::<Message>::from_iter(blocks.into_iter());
    assert_eq!(unique_blocks.len(), expected_blocks);

    // Test the number of expected aggregates per slot.
    let aggregates = g.get_msg(test_slot, MsgType::AggregateAndProofAttestation);
    assert_eq!(aggregates.len(), expected_aggregates);
    let unique_aggregates = HashSet::<Message>::from_iter(aggregates.into_iter());
    assert_eq!(unique_aggregates.len(), expected_aggregates);

    // Test the number of attestations per slot and epoch.
    let epoch = test_slot.epoch(slots_per_epoch);
    let slot_iter = epoch.slot_iter(slots_per_epoch);

    let mut total_attestations = HashSet::with_capacity(total_validators as usize);
    for current_slot in slot_iter {
        let slot_attestations = g.get_msg(current_slot, MsgType::Attestation);
        assert!(
            slot_attestations
                .len()
                .abs_diff(expected_attestations_per_slot)
                <= 1 // tolerance
        );
        let unique_slot_attestations = HashSet::<Message>::from_iter(slot_attestations.into_iter());
        assert!(
            unique_slot_attestations
                .len()
                .abs_diff(expected_attestations_per_slot)
                <= 1 // tolerance
        );
        total_attestations.extend(unique_slot_attestations.into_iter());
    }

    assert_eq!(total_attestations.len(), total_validators as usize);
}

#[tokio::test]
async fn test_timings() {}
