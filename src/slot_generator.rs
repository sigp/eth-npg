use std::collections::HashSet;

use slot_clock::Slot;

#[derive(PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
pub struct ValId(pub u64);

impl std::ops::Deref for ValId {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
pub struct Subnet(pub u64);

impl std::ops::Deref for Subnet {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Debug for ValId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("V{}", self.0))
    }
}

impl std::fmt::Debug for Subnet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("S{}", self.0))
    }
}

pub struct SlotGenerator {
    /// Epoch definition.
    slots_per_epoch: u64,
    /// Number of attestation subnets to split validators.
    attestation_subnets: u64,
    /// Number of validators to include in each sync subnet.
    sync_subnet_size: u64,
    /// Number of subcommittees to split members of the sync committee.
    sync_committee_subnets: u64,
    /// Number of validators to designate as aggregators in the sync committee and attestation
    /// subnets.
    target_aggregators: u64,
    /// Number of validators in the network.
    total_validators: u64,
    /// GCD(total_validators, attestation_subnets) == 1.
    att_subnets_is_relative: bool,
}

impl SlotGenerator {
    pub fn new(
        slots_per_epoch: u64,
        attestation_subnets: u64,
        sync_subnet_size: u64,
        sync_committee_subnets: u64,
        target_aggregators: u64,
        total_validators: u64,
    ) -> Self {
        fn gcd(mut a: u64, mut b: u64) -> u64 {
            while b > 0 {
                (a, b) = (b, a % b);
            }
            a
        }

        let att_subnets_is_relative = gcd(attestation_subnets, slots_per_epoch) == 1;
        Self {
            slots_per_epoch,
            attestation_subnets,
            sync_subnet_size,
            sync_committee_subnets,
            target_aggregators,
            total_validators,
            att_subnets_is_relative,
        }
    }

    pub fn get_blocks(&self, slot: Slot, validators: &HashSet<ValId>) -> Option<ValId> {
        let proposer = ValId(slot.as_u64() % self.total_validators);
        validators.contains(&proposer).then_some(proposer)
    }

    pub fn get_attestations<'a>(
        &'a self,
        slot: Slot,
        validators: &'a HashSet<ValId>,
    ) -> impl Iterator<Item = (ValId, Subnet)> + 'a {
        let epoch = slot.epoch(self.slots_per_epoch).as_u64();
        let slot = slot.as_u64();
        validators.iter().filter_map(move |val_id| {
            // shake the val id using the epoch
            let shaked_val_id = val_id.overflowing_add(epoch).0 % self.total_validators;
            // assign to one of the committees
            let subnet = Subnet(shaked_val_id % self.attestation_subnets);
            // assign attesters using the slot
            let is_attester = (shaked_val_id
                + if self.att_subnets_is_relative {
                    0
                } else {
                    shaked_val_id / self.attestation_subnets
                })
                % self.slots_per_epoch
                == slot % self.slots_per_epoch;
            is_attester.then_some((*val_id, subnet))
        })
    }

    pub fn get_aggregates<'a>(
        &'a self,
        slot: Slot,
        validators: &'a HashSet<ValId>,
    ) -> impl Iterator<Item = (ValId, Subnet)> + 'a {
        let epoch = slot.epoch(self.slots_per_epoch).as_u64();
        validators.iter().filter_map(move |val_id| {
            // shake the val id using the epoch
            let shaked_val_id = val_id.overflowing_add(epoch).0 % self.total_validators;
            // assign to one of the committees
            let subnet = Subnet(shaked_val_id % self.attestation_subnets);
            // get an id inside the committee
            let idx_in_commitee = shaked_val_id / self.attestation_subnets;
            let is_aggregator = (idx_in_commitee / self.target_aggregators) == 0;
            is_aggregator.then_some((*val_id, subnet))
        })
    }

    pub fn get_sync_committee_messages<'a>(
        &'a self,
        slot: Slot,
        validators: &'a HashSet<ValId>,
    ) -> impl Iterator<Item = (ValId, Subnet)> + 'a {
        let epoch = slot.epoch(self.slots_per_epoch).as_u64();
        let sync_committee_period = epoch / crate::EPOCHS_PER_SYNC_COMMITTEE_PERIOD;
        validators.iter().filter_map(move |val_id| {
            // shake the val id using the sync_committee_period and move it back to
            // the validator ids range.
            let shaked_val_id =
                val_id.overflowing_add(sync_committee_period).0 % self.total_validators;
            let sync_committee_size = self.sync_subnet_size * self.sync_committee_subnets;
            let in_commitee = shaked_val_id / sync_committee_size == 0;
            in_commitee.then(|| {
                let subnet = Subnet(shaked_val_id % self.sync_committee_subnets);
                (*val_id, subnet)
            })
        })
    }

    pub fn get_sync_committee_aggregates<'a>(
        &'a self,
        slot: Slot,
        validators: &'a HashSet<ValId>,
    ) -> impl Iterator<Item = (ValId, Subnet)> + 'a {
        let epoch = slot.epoch(self.slots_per_epoch).as_u64();
        let sync_committee_period = epoch / crate::EPOCHS_PER_SYNC_COMMITTEE_PERIOD;
        validators.iter().filter_map(move |val_id| {
            // shake the val id using the sync_committee_period and move it back to
            // the validator ids range.
            let shaked_val_id =
                val_id.overflowing_add(sync_committee_period).0 % self.total_validators;
            let sync_committee_size = self.sync_subnet_size * self.sync_committee_subnets;
            let in_commitee = shaked_val_id / sync_committee_size == 0;
            let subnet = Subnet(shaked_val_id % self.sync_committee_subnets);
            let id_in_subnet = shaked_val_id / self.sync_committee_subnets;
            let is_aggregator = (id_in_subnet / self.target_aggregators) == 0;
            (in_commitee && is_aggregator).then(|| (*val_id, subnet))
        })
    }
}
