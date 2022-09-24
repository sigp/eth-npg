use std::{collections::HashSet, time::Duration};

use slot_clock::{Slot, SlotClock, SystemTimeSlotClock};

use crate::ValId;

use super::Generator;

const DEFAULT_SLOT_DURATION_SECONDS: u64 = 12;
const DEFAULT_ATTESTATION_SUBNETS: usize = 64;
const DEFAULT_TARGET_AGGREGATORS: usize = 16;
const DEFAULT_SYNC_COMMITTEE_SIZE: usize = 512;
const DEFAULT_SYNC_COMMITTEE_SUBNETS: usize = 4;
const DEFAULT_SLOTS_PER_EPOCH: usize = 32;

#[derive(Default)]
pub struct GeneratorBuilder {
    slot_clock: Option<SystemTimeSlotClock>,
    attestation_subnets: Option<usize>,
    target_aggregators: Option<usize>,
    sync_subnet_size: Option<usize>,
    sync_committee_subnets: Option<usize>,
    slots_per_epoch: Option<usize>,
    total_validators: Option<usize>,
}

/// Validated parameters for a Generator.
/// Models a partially validated builder useful to reuse params.
#[derive(Clone)]
pub struct GeneratorParams {
    /// Slot clock based on system time.
    slot_clock: SystemTimeSlotClock,
    /// Epoch definition.
    slots_per_epoch: usize,
    /// Number of attestation subnets to split validators.
    attestation_subnets: usize,
    /// Number of validators to include in the each sync subnet.
    sync_subnet_size: usize,
    /// Number of subcommittees to split members of the sync committee.
    sync_committee_subnets: usize,
    /// Number of validators to designate as aggregators in the sync committee and attestation
    /// subnets.
    target_aggregators: usize,
    /// Number of validators in the network.
    total_validators: usize,
}

impl GeneratorParams {
    pub fn new(
        slot_clock: SystemTimeSlotClock,
        slots_per_epoch: usize,
        attestation_subnets: usize,
        sync_subnet_size: usize,
        sync_committee_subnets: usize,
        target_aggregators: usize,
        total_validators: usize,
    ) -> Result<Self, &'static str> {
        if slots_per_epoch == 0 {
            // Epochs are defined
            Err("slots_per_epoch should be positive")
        } else if total_validators == 0 {
            // Network is not empty
            Err("total_validators must be positive")
        } else if attestation_subnets == 0 {
            // Attestation subnets must make sense
            Err("attestation_subnets must be positive")
        } else if sync_committee_subnets == 0 {
            // Sync subnets make sense
            Err("sync_committee_subnets must be positive")
        } else if sync_subnet_size
            .checked_mul(sync_committee_subnets)
            .ok_or("sync committee size is too large")?
            > total_validators
        {
            // There must be enough validators to cover the sync committee.
            Err("not enought validators to reach the sync committees size")
        } else if target_aggregators
            .checked_mul(attestation_subnets)
            .ok_or("total attestation aggregators across the network is too large")?
            > total_validators
        {
            // There must be enough validators to cover the aggregators requirements.
            Err("not enough validators to reach the target aggregators in the attestation subnets")
        } else if target_aggregators
            .checked_mul(sync_committee_subnets)
            .ok_or("target sync aggregators across the network is too large")?
            > total_validators
        {
            // There must be enough validators to cover the aggregators requirements.
            Err("not enough validators to reach the target aggregators in the sync committees")
        } else {
            Ok(GeneratorParams {
                slot_clock,
                slots_per_epoch,
                attestation_subnets,
                sync_subnet_size,
                sync_committee_subnets,
                target_aggregators,
                total_validators,
            })
        }
    }

    pub fn build(self, validators: HashSet<ValId>) -> Result<Generator, &'static str> {
        let Self {
            slot_clock,
            slots_per_epoch,
            attestation_subnets,
            sync_subnet_size,
            sync_committee_subnets,
            target_aggregators,
            total_validators,
        } = self;
        if validators.iter().any(|val_id| val_id >= &total_validators) {
            return Err("validator ids must go up to total_validators - 1");
        }

        let next_slot = slot_clock
            .duration_to_next_slot()
            .expect("system clock is unlikely to fail");
        Ok(Generator {
            slot_clock,
            slots_per_epoch,
            attestation_subnets,
            validators,
            total_validators,
            sync_subnet_size,
            sync_committee_subnets,
            target_aggregators,
            queued_messages: Default::default(),
            next_slot: Box::pin(tokio::time::sleep(next_slot)),
        })
    }

    pub fn slots_per_epoch(&self) -> usize {
        self.slots_per_epoch
    }

    pub fn attestation_subnets(&self) -> usize {
        self.attestation_subnets
    }

    pub fn sync_subnet_size(&self) -> usize {
        self.sync_subnet_size
    }

    pub fn sync_committee_subnets(&self) -> usize {
        self.sync_committee_subnets
    }

    pub fn target_aggregators(&self) -> usize {
        self.target_aggregators
    }

    pub fn total_validators(&self) -> usize {
        self.total_validators
    }
}

impl GeneratorBuilder {
    /// Slot clock based on system time.
    pub fn slot_clock(
        &mut self,
        genesis_slot: usize,
        genesis_duration: Duration,
        slot_duration: Duration,
    ) -> &mut Self {
        self.slot_clock = Some(SystemTimeSlotClock::new(
            Slot::new(genesis_slot as u64),
            genesis_duration,
            slot_duration,
        ));
        self
    }

    /// Number of attestation subnets to split validators.
    pub fn attestation_subnets(&mut self, attestation_subnets: usize) -> &mut Self {
        self.attestation_subnets = Some(attestation_subnets);
        self
    }

    /// Number of validators to designate as aggregators in the sync committee and attestation
    /// subnets.
    pub fn target_aggregators(&mut self, aggregators: usize) -> &mut Self {
        self.target_aggregators = Some(aggregators);
        self
    }

    /// Number of validators to include in the each sync subnet.
    pub fn sync_subnet_size(&mut self, sync_subnet_size: usize) -> &mut Self {
        self.sync_subnet_size = Some(sync_subnet_size);
        self
    }

    /// Number of subcommittees to split members of the sync committee.
    pub fn sync_committee_subnets(&mut self, sync_committee_subnets: usize) -> &mut Self {
        self.sync_committee_subnets = Some(sync_committee_subnets);
        self
    }

    /// Epoch definition.
    pub fn slots_per_epoch(&mut self, slots_per_epoch: usize) -> &mut Self {
        self.slots_per_epoch = Some(slots_per_epoch);
        self
    }

    /// Number of validators in the network.
    pub fn total_validators(&mut self, total_validators: usize) -> &mut Self {
        self.total_validators = Some(total_validators);
        self
    }

    pub fn build_params(&mut self) -> Result<GeneratorParams, &'static str> {
        let GeneratorBuilder {
            slot_clock,
            attestation_subnets,
            target_aggregators,
            sync_subnet_size,
            sync_committee_subnets,
            slots_per_epoch,
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

        GeneratorParams::new(
            slot_clock,
            slots_per_epoch,
            attestation_subnets,
            sync_subnet_size,
            sync_committee_subnets,
            target_aggregators,
            total_validators,
        )
    }

    pub fn build(&mut self, validators: HashSet<ValId>) -> Result<Generator, &'static str> {
        self.build_params()?.build(validators)
    }
}
