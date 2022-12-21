use std::{
    collections::{HashSet, VecDeque},
    pin::Pin,
    task::Poll,
};

//use futures::{stream::Stream, Future};
use futures::stream::Stream;
use slot_clock::{Slot, SlotClock, SystemTimeSlotClock};
use slot_generator::{SlotGenerator, Subnet, ValId};
use strum::{EnumIter, IntoEnumIterator};
// use tokio::time::{sleep, Sleep};

pub mod builder;
pub mod sizes;
pub mod slot_generator;
#[cfg(test)]
mod tests;

#[derive(EnumIter, Debug, strum::Display, Clone, Copy)]
#[strum(serialize_all = "kebab_case")]
pub enum MsgType {
    BeaconBlock,
    AggregateAndProofAttestation,
    Attestation,
    SignedContributionAndProof,
    SyncCommitteeMessage,
}

pub struct Generator {
    /// Slot clock based on system time.
    slot_clock: SystemTimeSlotClock,
    /// Slot messages generator.
    slot_generator: SlotGenerator,
    /// Validator managed by this node.
    validators: HashSet<ValId>,
    /// Messages pending to be returned.
    queued_messages: VecDeque<Message>,
    /// Slot interval.
    interval: tokio::time::Interval,
    /// Slot interval count. The interval occurs every 1/3 of a slot. So we keep track where we are
    interval_count: u8,
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum Message {
    BeaconBlock {
        proposer: ValId,
        slot: Slot,
    },
    AggregateAndProofAttestation {
        aggregator: ValId,
        subnet: Subnet,
        slot: Slot,
    },
    Attestation {
        attester: ValId,
        subnet: Subnet,
        slot: Slot,
    },
    SignedContributionAndProof {
        validator: ValId,
        subnet: Subnet,
        slot: Slot,
    },
    SyncCommitteeMessage {
        validator: ValId,
        subnet: Subnet,
        slot: Slot,
    },
}

const EPOCHS_PER_SYNC_COMMITTEE_PERIOD: u64 = 256;

impl Generator {
    pub fn builder() -> builder::GeneratorBuilder {
        builder::GeneratorBuilder::default()
    }

    /// Time since last slot
    pub fn time_since_last_slot(&self) -> std::time::Duration {
        self.slot_clock.slot_duration().saturating_sub(
            self.slot_clock
                .duration_to_next_slot()
                .unwrap_or(std::time::Duration::ZERO),
        )
    }

    // Occurs every slot
    fn queue_slot_msgs(&mut self, current_slot: Slot) {
        for msg_type in MsgType::iter() {
            match msg_type {
                MsgType::BeaconBlock => self.queued_messages.extend(
                    self.slot_generator
                        .get_blocks(current_slot, &self.validators)
                        .map(|proposer| Message::BeaconBlock {
                            proposer,
                            slot: current_slot,
                        }),
                ),
                MsgType::AggregateAndProofAttestation => self.queued_messages.extend(
                    self.slot_generator
                        .get_aggregates(current_slot, &self.validators)
                        .map(
                            |(aggregator, subnet)| Message::AggregateAndProofAttestation {
                                aggregator,
                                subnet,
                                slot: current_slot,
                            },
                        ),
                ),
                MsgType::Attestation => self.queued_messages.extend(
                    self.slot_generator
                        .get_attestations(current_slot, &self.validators)
                        .map(|(attester, subnet)| Message::Attestation {
                            attester,
                            subnet,
                            slot: current_slot,
                        }),
                ),
                MsgType::SignedContributionAndProof => self.queued_messages.extend(
                    self.slot_generator
                        .get_sync_committee_aggregates(current_slot, &self.validators)
                        .map(|(validator, subnet)| Message::SignedContributionAndProof {
                            validator,
                            subnet,
                            slot: current_slot,
                        }),
                ),
                MsgType::SyncCommitteeMessage => self.queued_messages.extend(
                    self.slot_generator
                        .get_sync_committee_messages(current_slot, &self.validators)
                        .map(|(validator, subnet)| Message::SyncCommitteeMessage {
                            validator,
                            subnet,
                            slot: current_slot,
                        }),
                ),
            }
        }
    }

    // Occurs every 2/3 of a slot
    fn queue_aggregate_msgs(&mut self, current_slot: Slot) {
        for msg_type in MsgType::iter() {
            match msg_type {
                MsgType::BeaconBlock => {}
                MsgType::AggregateAndProofAttestation => self.queued_messages.extend(
                    self.slot_generator
                        .get_aggregates(current_slot, &self.validators)
                        .map(
                            |(aggregator, subnet)| Message::AggregateAndProofAttestation {
                                aggregator,
                                subnet,
                                slot: current_slot,
                            },
                        ),
                ),
                MsgType::Attestation => {}
                MsgType::SignedContributionAndProof => self.queued_messages.extend(
                    self.slot_generator
                        .get_sync_committee_aggregates(current_slot, &self.validators)
                        .map(|(validator, subnet)| Message::SignedContributionAndProof {
                            validator,
                            subnet,
                            slot: current_slot,
                        }),
                ),
                MsgType::SyncCommitteeMessage => {}
            }
        }
    }
}

impl Stream for Generator {
    type Item = Message;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        // If there were any messages remaining from the current slot, return them.
        if let Some(msg) = self.queued_messages.pop_front() {
            return Poll::Ready(Some(msg));
        }

        if self.interval.poll_tick(cx).is_ready() {
            self.interval_count += 1;
            if self.interval_count == 3 {
                self.interval_count = 0;
                let current_slot = self.slot_clock.now().expect("Slot exists");
                self.queue_slot_msgs(current_slot);
            } else if self.interval_count == 2 {
                // Aggregates get sent 2/3 of the way through the slot
                let current_slot = self.slot_clock.now().expect("Slot exists");
                self.queue_aggregate_msgs(current_slot);
            }
        }

        // If there were any messages remaining from the current slot, return them.
        if let Some(msg) = self.queued_messages.pop_front() {
            return Poll::Ready(Some(msg));
        }

        Poll::Pending
    }
}
