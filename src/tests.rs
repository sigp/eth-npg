use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::slot_generator::{SlotGenerator, Subnet, ValId};

use slot_clock::Slot;

#[test]
fn test_attestations() {
    let test_slot = 0;
    let slots_per_epoch = 12;
    let total_validators = 20000;
    let attestation_subnets = 6;
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

    let mut epoch_attesters = HashSet::with_capacity(total_validators as usize);
    let expected_slot_atts = (total_validators / slots_per_epoch) as usize;
    let epected_slot_subnet_atts =
        (total_validators / slots_per_epoch / attestation_subnets) as usize;
    for current_slot in slot_iter {
        let mut subnet_messages = BTreeMap::<Subnet, BTreeSet<ValId>>::default();

        for (val_id, subnet) in slot_generator.get_attestations(current_slot, &all_validators) {
            let is_new = epoch_attesters.insert(val_id);
            assert!(is_new, "each validator attests just once per epoch");
            subnet_messages.entry(subnet).or_default().insert(val_id);
        }

        assert_eq!(
            subnet_messages.len(),
            attestation_subnets as usize,
            "all subnets should be attested on every epoch"
        );

        for (_subnet, msgs) in subnet_messages.iter() {
            assert!(msgs.len().abs_diff(epected_slot_subnet_atts) <= 1);
        }

        let slot_atts_count: usize = subnet_messages.values().map(|vals| vals.len()).sum();
        assert!(
            slot_atts_count.abs_diff(expected_slot_atts) <= 1,
            "unexpected number of messages per slot. Expected {} actual {}",
            expected_slot_atts,
            slot_atts_count
        );
    }
    assert_eq!(epoch_attesters.len(), total_validators as usize);
}

#[test]
fn test_aggregates() {
    let test_slot = 0;
    let slots_per_epoch = 32;
    let total_validators = 765878;
    let attestation_subnets = 64;
    let aggregators = 16;
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

    let mut all_aggregators = HashSet::with_capacity((aggregators * attestation_subnets) as usize);
    let mut per_subnet_aggregators = BTreeMap::<Subnet, usize>::default();
    let aggregates = slot_generator.get_aggregates(Slot::new(test_slot), &all_validators);

    for (val_id, subnet) in aggregates {
        let is_new = all_aggregators.insert(val_id);
        assert!(is_new);
        *per_subnet_aggregators.entry(subnet).or_default() += 1;
    }

    assert_eq!(per_subnet_aggregators.len(), attestation_subnets as usize);
    for aggregates_in_subnet in per_subnet_aggregators.values() {
        assert_eq!(aggregators as usize, *aggregates_in_subnet)
    }
}

#[test]
fn test_sync_messages() {
    let test_slot = 0;
    let slots_per_epoch = 32;
    let total_validators = 765878;
    let attestation_subnets = 64;
    let aggregators = 16;
    let sync_subnet_size = 128;
    let sync_subnets = 4;

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

    let sync_messages =
        slot_generator.get_sync_committee_messages(Slot::new(test_slot), &all_validators);
    let mut per_subnet_messages = HashMap::<Subnet, usize>::with_capacity(sync_subnets as usize);
    for (_val_id, subnet) in sync_messages {
        *per_subnet_messages.entry(subnet).or_default() += 1;
    }
    assert_eq!(per_subnet_messages.len(), sync_subnets as usize);
    assert!(per_subnet_messages
        .values()
        .all(|count| *count == sync_subnet_size as usize));
}

#[test]
fn test_sync_aggregates() {
    let test_slot = 0;
    let slots_per_epoch = 32;
    let total_validators = 20;
    let attestation_subnets = 64;
    let aggregators = 3;
    let sync_subnet_size = 6;
    let sync_subnets = 4;

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

    let mut subnet_aggregators = BTreeMap::<Subnet, BTreeSet<ValId>>::default();
    let sync_messages =
        slot_generator.get_sync_committee_aggregates(Slot::new(test_slot), &all_validators);
    for (val_id, subnet) in sync_messages {
        subnet_aggregators.entry(subnet).or_default().insert(val_id);
    }
    assert_eq!(subnet_aggregators.len(), sync_subnets as usize);
    assert!(subnet_aggregators
        .values()
        .all(|vals| vals.len() == aggregators as usize));
}
