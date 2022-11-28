use std::{collections::HashSet, time::Duration};

use slot_clock::{Slot, SlotClock, SystemTimeSlotClock};

use crate::{slot_generator::SlotGenerator, ValId};

use super::Generator;

const DEFAULT_SLOT_DURATION_SECONDS: u64 = 12;
const DEFAULT_ATTESTATION_SUBNETS: u64 = 64;
const DEFAULT_TARGET_AGGREGATORS: u64 = 16;
const DEFAULT_SYNC_COMMITTEE_SIZE: u64 = 512;
const DEFAULT_SYNC_COMMITTEE_SUBNETS: u64 = 4;
const DEFAULT_SLOTS_PER_EPOCH: u64 = 32;

#[derive(Default)]
pub struct GeneratorBuilder {
    slot_clock: Option<SystemTimeSlotClock>,
    attestation_subnets: Option<u64>,
    target_aggregators: Option<u64>,
    sync_subnet_size: Option<u64>,
    sync_committee_subnets: Option<u64>,
    slots_per_epoch: Option<u64>,
    total_validators: Option<u64>,
}

impl GeneratorBuilder {
    /// Slot clock based on system time.
    pub fn slot_clock(
        &mut self,
        genesis_slot: u64,
        genesis_duration: Duration,
        slot_duration: Duration,
    ) -> &mut Self {
        self.slot_clock = Some(SystemTimeSlotClock::new(
            Slot::new(genesis_slot),
            genesis_duration,
            slot_duration,
        ));
        self
    }

    /// Number of attestation subnets to split validators.
    pub fn attestation_subnets(&mut self, attestation_subnets: u64) -> &mut Self {
        self.attestation_subnets = Some(attestation_subnets);
        self
    }

    /// Number of validators to designate as aggregators in the sync committee and attestation
    /// subnets.
    pub fn target_aggregators(&mut self, aggregators: u64) -> &mut Self {
        self.target_aggregators = Some(aggregators);
        self
    }

    /// Number of validators to include in the each sync subnet.
    pub fn sync_subnet_size(&mut self, sync_subnet_size: u64) -> &mut Self {
        self.sync_subnet_size = Some(sync_subnet_size);
        self
    }

    /// Number of subcommittees to split members of the sync committee.
    pub fn sync_committee_subnets(&mut self, sync_committee_subnets: u64) -> &mut Self {
        self.sync_committee_subnets = Some(sync_committee_subnets);
        self
    }

    /// Epoch definition.
    pub fn slots_per_epoch(&mut self, slots_per_epoch: u64) -> &mut Self {
        self.slots_per_epoch = Some(slots_per_epoch);
        self
    }

    /// Number of validators in the network.
    pub fn total_validators(&mut self, total_validators: u64) -> &mut Self {
        self.total_validators = Some(total_validators);
        self
    }

    pub fn build(&self, validators: HashSet<ValId>) -> Result<Generator, &'static str> {
        let Self {
            slot_clock,
            slots_per_epoch,
            attestation_subnets,
            sync_subnet_size,
            sync_committee_subnets,
            target_aggregators,
            total_validators,
        } = self;

        let slot_clock = slot_clock.clone().unwrap_or_else(|| {
            SystemTimeSlotClock::new(
                Slot::new(0),
                Duration::ZERO,
                Duration::from_secs(DEFAULT_SLOT_DURATION_SECONDS),
            )
        });

        let total_validators = total_validators.ok_or("total_validators not set")?;
        let attestation_subnets = attestation_subnets.unwrap_or(DEFAULT_ATTESTATION_SUBNETS);
        let target_aggregators = target_aggregators.unwrap_or(DEFAULT_TARGET_AGGREGATORS);
        let sync_subnet_size = sync_subnet_size.unwrap_or(DEFAULT_SYNC_COMMITTEE_SIZE);
        let sync_committee_subnets =
            sync_committee_subnets.unwrap_or(DEFAULT_SYNC_COMMITTEE_SUBNETS);
        let slots_per_epoch = slots_per_epoch.unwrap_or(DEFAULT_SLOTS_PER_EPOCH);

        if validators.iter().any(|val_id| total_validators <= **val_id) {
            return Err("validator ids must go up to total_validators - 1");
        }

        if slots_per_epoch == 0 {
            // Epochs are defined
            return Err("slots_per_epoch should be positive");
        }
        if total_validators == 0 {
            // Network is not empty
            return Err("total_validators must be positive");
        }
        if attestation_subnets == 0 {
            // Attestation subnets must make sense
            return Err("attestation_subnets must be positive");
        }
        if sync_committee_subnets == 0 {
            // Sync subnets make sense
            return Err("sync_committee_subnets must be positive");
        }
        if sync_subnet_size
            .checked_mul(sync_committee_subnets)
            .ok_or("sync committee size is too large")?
            > total_validators
        {
            // There must be enough validators to cover the sync committee.
            return Err("not enought validators to reach the sync committees size");
        }
        if target_aggregators
            .checked_mul(attestation_subnets)
            .ok_or("total attestation aggregators across the network is too large")?
            > total_validators
        {
            // There must be enough validators to cover the aggregators requirements.
            return Err(
                "not enough validators to reach the target aggregators in the attestation subnets",
            );
        }
        if target_aggregators
            .checked_mul(sync_committee_subnets)
            .ok_or("target sync aggregators across the network is too large")?
            > total_validators
        {
            // There must be enough validators to cover the aggregators requirements.
            return Err(
                "not enough validators to reach the target aggregators in the sync committees",
            );
        }

        let next_slot = slot_clock
            .duration_to_next_slot()
            .expect("system clock is unlikely to fail");
        let slot_generator = SlotGenerator::new(
            slots_per_epoch,
            attestation_subnets,
            sync_subnet_size,
            sync_committee_subnets,
            target_aggregators,
            total_validators,
        );

        // Slot interval
        let interval = tokio::time::interval_at(
            tokio::time::Instant::now() + next_slot,
            slot_clock.slot_duration(),
        );

        Ok(Generator {
            slot_clock,
            slot_generator,
            validators,
            queued_messages: Default::default(),
            interval,
        })
    }
}
