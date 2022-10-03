use slot_clock::Slot;

use crate::{builder::GeneratorParams, ValId};

fn get_slot_attestations<'a, I: Iterator<Item = ValId> + 'a>(
    validators: I,
    current_slot: Slot,
    params: GeneratorParams,
) -> impl Iterator<Item = (ValId, usize)> + 'a {
    let slot = current_slot.as_usize();
    let epoch = current_slot
        .epoch(params.slots_per_epoch() as u64)
        .as_usize();
    validators.filter_map(move |val_id| {
        // shake the val id using the epoch
        let shaked_val_id = val_id.overflowing_add(epoch).0;
        // assign to one of the committees
        let attnet = shaked_val_id % params.attestation_subnets();
        let id_in_attnet =
            (shaked_val_id % params.total_validators()) / params.attestation_subnets();
        // assign attesters using the slot
        let is_attester =
            id_in_attnet % params.slots_per_epoch() == slot % params.slots_per_epoch();
        is_attester.then_some((val_id, attnet))
    })
}

#[cfg(test)]
mod tests {
    use types::validator;

    use crate::Generator;

    use super::*;

    #[test]
    fn test_get_slot_attestations() {
        let node_count = 1;
        let test_slot = 0;
        let slots_per_epoch = 2;
        let total_validators = 50;
        let attestation_subnets = 8;
        let aggregators = 1;
        let sync_subnet_size = 1;
        let sync_subnets = 1;

        let mut builder = Generator::builder();
        let params = builder
            .slots_per_epoch(slots_per_epoch)
            .total_validators(total_validators)
            .attestation_subnets(attestation_subnets)
            .target_aggregators(aggregators)
            .sync_committee_subnets(sync_subnets)
            .sync_subnet_size(sync_subnet_size)
            .build_params()
            .expect("right params");

        let validators = (0..params.total_validators()).into_iter();
        let test_slot = Slot::new(test_slot);
        let slot_attestations = get_slot_attestations(validators, test_slot, params);
    }
}
