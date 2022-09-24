use std::{
    collections::{HashSet, VecDeque},
    pin::Pin,
    task::Poll,
};

use futures::{stream::Stream, Future};
use slot_clock::{Slot, SlotClock, SystemTimeSlotClock};
use strum::{EnumIter, IntoEnumIterator};
use tokio::time::{sleep, Sleep};

mod builder;
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
    /// Epoch definition.
    slots_per_epoch: usize,
    /// Number of attestation subnets to split validators.
    attestation_subnets: usize,
    /// Number of validators to include in each sync subnet.
    sync_subnet_size: usize,
    /// Number of subcommittees to split members of the sync committee.
    sync_committee_subnets: usize,
    /// Number of validators to designate as aggregators in the sync committee and attestation
    /// subnets.
    target_aggregators: usize,
    /// Number of validators in the network.
    total_validators: usize,
    /// Validator managed by this node.
    validators: HashSet<usize>,
    /// Messages pending to be returned.
    queued_messages: VecDeque<Message>,
    /// Duration to the next slot.
    next_slot: Pin<Box<Sleep>>,
}

pub type ValId = usize;

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum Message {
    BeaconBlock { proposer: ValId },
    AggregateAndProofAttestation { aggregator: ValId, committee: usize },
    Attestation { attester: ValId, committee: usize },
    SignedContributionAndProof { validator: ValId, committee: usize },
    SyncCommitteeMessage { validator: ValId, committee: usize },
}

const EPOCHS_PER_SYNC_COMMITTEE_PERIOD: usize = 256;

impl Generator {
    pub fn builder() -> builder::GeneratorBuilder {
        builder::GeneratorBuilder::default()
    }

    pub fn get_attestations(
        &self,
        current_slot: Slot,
    ) -> impl Iterator<Item = (ValId, usize)> + '_ {
        let slot = current_slot.as_usize();
        let epoch = current_slot.epoch(self.slots_per_epoch as u64).as_usize();
        self.validators.iter().filter_map(move |val_id| {
            // shake the val id using the epoch
            let shaked_val_id = val_id.overflowing_add(epoch).0;
            // assign to one of the committees
            let committee = shaked_val_id % self.attestation_subnets;
            // assign attesters using the slot
            let is_attester = val_id.overflowing_add(slot).0 % self.slots_per_epoch == 0;
            is_attester.then_some((*val_id, committee))
        })
    }

    pub fn get_msg(&self, current_slot: Slot, kind: MsgType) -> Vec<Message> {
        let slot = current_slot.as_usize();
        let epoch = current_slot.epoch(self.slots_per_epoch as u64).as_usize();
        match kind {
            MsgType::BeaconBlock => {
                // return a block if we have the validator that should send a block
                let proposer = slot % self.total_validators;
                if self.validators.contains(&proposer) {
                    vec![Message::BeaconBlock { proposer }]
                } else {
                    vec![]
                }
            }
            MsgType::AggregateAndProofAttestation => {
                self.validators
                    .iter()
                    .filter_map(|val_id| {
                        // shake the val id using the epoch
                        let shaked_val_id = val_id.overflowing_add(epoch).0;
                        // assign to one of the committees
                        let committee = shaked_val_id % self.attestation_subnets;
                        // get an id on the range of existing validator ids and use it to get an id
                        // inside the committee
                        let idx_in_commitee =
                            (shaked_val_id % self.total_validators) / self.attestation_subnets;
                        let is_aggregator = idx_in_commitee / self.target_aggregators == 0;
                        is_aggregator.then_some(Message::AggregateAndProofAttestation {
                            aggregator: *val_id,
                            committee,
                        })
                    })
                    .collect()
            }
            MsgType::Attestation => self
                .get_attestations(current_slot)
                .map(|(val_id, attnet)| Message::Attestation {
                    attester: val_id,
                    committee: attnet,
                })
                .collect(),
            MsgType::SyncCommitteeMessage => {
                let sync_committee_period = epoch / EPOCHS_PER_SYNC_COMMITTEE_PERIOD;
                self.validators
                    .iter()
                    .filter_map(|val_id| {
                        // shake the val id using the sync_committee_period and move it back to
                        // the validator ids range.
                        let shaked_val_id =
                            val_id.overflowing_add(sync_committee_period).0 % self.total_validators;
                        let sync_committee_size =
                            self.sync_subnet_size * self.sync_committee_subnets;
                        let in_commitee = shaked_val_id / sync_committee_size == 0;
                        in_commitee.then(|| {
                            let idx_in_commitee = shaked_val_id % sync_committee_size;
                            let committee = idx_in_commitee % self.sync_committee_subnets;
                            Message::SyncCommitteeMessage {
                                validator: *val_id,
                                committee,
                            }
                        })
                    })
                    .collect()
            }
            MsgType::SignedContributionAndProof => {
                let sync_committee_period = epoch / EPOCHS_PER_SYNC_COMMITTEE_PERIOD;
                self.validators
                    .iter()
                    .filter_map(|val_id| {
                        // shake the val id using the sync_committee_period and move it back to
                        // the validator ids range.
                        let shaked_val_id =
                            val_id.overflowing_add(sync_committee_period).0 % self.total_validators;
                        let sync_committee_size =
                            self.sync_subnet_size * self.sync_committee_subnets;
                        let in_commitee = shaked_val_id / sync_committee_size == 0;
                        if !in_commitee {
                            return None;
                        }
                        let idx_in_commitee = shaked_val_id % sync_committee_size;
                        let committee = idx_in_commitee % self.sync_committee_subnets;
                        let idx_in_subcommittee = idx_in_commitee / self.sync_committee_subnets;
                        let is_aggregator = (idx_in_subcommittee.overflowing_add(slot).0
                            % self.sync_committee_subnets)
                            / self.target_aggregators
                            == 0;
                        is_aggregator.then_some(Message::SignedContributionAndProof {
                            validator: *val_id,
                            committee,
                        })
                    })
                    .collect()
            }
        }
    }

    fn queue_slot_msgs(&mut self, current_slot: Slot) {
        for msg_type in MsgType::iter() {
            let msgs = self.get_msg(current_slot, msg_type);
            self.queued_messages.extend(msgs);
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

        if self.next_slot.as_mut().poll(cx).is_ready() {
            let current_slot = self.slot_clock.now().unwrap();
            self.queue_slot_msgs(current_slot);

            let duration_to_next_slot = self.slot_clock.duration_to_next_slot().unwrap();
            self.next_slot = Box::pin(sleep(duration_to_next_slot));
            // We either have messages to return or need to poll the sleep
            cx.waker().wake_by_ref();
        }

        Poll::Pending
    }
}
