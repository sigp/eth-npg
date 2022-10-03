use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::slot_generator::{SlotGenerator, Subnet, ValId};

use quickcheck_macros::quickcheck;
use rand::seq::SliceRandom;
use slot_clock::Slot;

#[test]
fn sanity_check_2() {
    let test_slot = 0;
    let slots_per_epoch = 2;
    let total_validators = 17;
    let attestation_subnets = 7;
    let aggregators = 1;
    let sync_subnet_size = 1;
    let sync_subnets = 1;
    let slot_generator = SlotGenerator::new(
        slots_per_epoch,
        attestation_subnets,
        sync_subnet_size,
        sync_subnets,
        aggregators,
        total_validators,
    );

    let all_validators = (0..total_validators)
        .map(super::slot_generator::ValId)
        .collect();

    // Test the number of attestations per slot and epoch.
    // per slot, the total messages sent will be V/32.
    // So on each subnet we should expect to see V/32/64
    let epoch = Slot::new(test_slot).epoch(slots_per_epoch);
    let slot_iter = epoch.slot_iter(slots_per_epoch);

    // let mut total_attestations = HashSet::with_capacity(total_validators as usize);
    let mut epoch_attesters = HashSet::with_capacity(total_validators as usize);
    let expected_slot_atts = (total_validators / slots_per_epoch) as usize;
    for current_slot in slot_iter {
        let mut subnet_messages = BTreeMap::<Subnet, BTreeSet<ValId>>::default();

        for (val_id, subnet) in slot_generator.get_attestations(current_slot, &all_validators) {
            let is_new = epoch_attesters.insert(val_id);
            assert!(is_new, "each validator attests just once per epoch");
            subnet_messages.entry(subnet).or_default().insert(val_id);
        }

        // assert_eq!(
        // subnet_messages.len(),
        // attestation_subnets as usize,
        // "all subnets should be attested on every epoch"
        // );

        println!("\nSLOT {current_slot}");
        println!("{subnet_messages:?}");

        let slot_atts_count: usize = subnet_messages.values().map(|vals| vals.len()).sum();
        // assert!(
        //     slot_atts_count.abs_diff(expected_slot_atts) <= 1,
        println!("expected {expected_slot_atts} got {slot_atts_count}")
        // );
    }
    assert_eq!(epoch_attesters.len(), total_validators as usize)
}
