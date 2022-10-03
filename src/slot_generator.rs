use std::{collections::HashSet, process::id};

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
    max_y: u64,
    /// Number of attestation subnets to split validators.
    max_x: u64,
    /// Number of validators to include in each sync subnet.
    sync_subnet_size: u64,
    /// Number of subcommittees to split members of the sync committee.
    sync_committee_subnets: u64,
    /// Number of validators to designate as aggregators in the sync committee and attestation
    /// subnets.
    target_aggregators: u64,
    /// Number of validators in the network.
    total_n: u64,
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
        Self {
            max_y: slots_per_epoch,
            max_x: attestation_subnets,
            sync_subnet_size,
            sync_committee_subnets,
            target_aggregators,
            total_n: total_validators,
        }
    }

    pub fn get_blocks(&self, slot: Slot, validators: &HashSet<ValId>) -> Option<ValId> {
        let proposer = ValId(slot.as_u64() % self.total_n);
        validators.contains(&proposer).then_some(proposer)
    }

    pub fn get_attestations<'a>(
        &'a self,
        slot: Slot,
        validators: &'a HashSet<ValId>,
    ) -> impl Iterator<Item = (ValId, Subnet)> + 'a {
        let epoch = slot.epoch(self.max_y).as_u64();
        let y = slot.as_u64();
        validators.iter().filter_map(move |val_id| {
            // shake the val id using the epoch
            let n = val_id.overflowing_add(epoch).0 % self.total_n;
            // assign to one of the committees
            let x = Subnet(n % self.max_x);
            // assign attesters using the slot
            let is_attester = n % self.max_y == y % self.max_y;
            is_attester.then_some((*val_id, x))
        })
    }

    pub fn get_aggregates<'a>(
        &'a self,
        slot: Slot,
        validators: &'a HashSet<ValId>,
    ) -> impl Iterator<Item = (ValId, Subnet)> + 'a {
        let epoch = slot.epoch(self.max_y).as_u64();
        validators.iter().filter_map(move |val_id| {
            // shake the val id using the epoch
            let shaked_val_id = val_id.overflowing_add(epoch).0;
            // assign to one of the committees
            let subnet = Subnet(shaked_val_id % self.max_x);
            // get an id on the range of existing validator ids and use it to get an id
            // inside the committee
            let idx_in_commitee = (shaked_val_id % self.total_n) / self.max_x;
            let is_aggregator = idx_in_commitee / self.target_aggregators == 0;
            is_aggregator.then_some((*val_id, subnet))
        })
    }

    pub fn get_sync_committee_aggregates(&self, slot: Slot, validators: &HashSet<ValId>) {}
    pub fn get_sync_committee_message(&self, slot: Slot, validators: &HashSet<ValId>) {}
}
