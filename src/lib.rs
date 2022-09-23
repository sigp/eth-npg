use std::{
    collections::{HashSet, VecDeque},
    convert::TryInto,
    fmt::format,
    marker::PhantomData,
    pin::Pin,
    sync::Arc,
    task::Poll,
    time::Duration,
};

use futures::{stream::Stream, Future, FutureExt};
use slot_clock::{SlotClock, SystemTimeSlotClock};
use strum::{EnumIter, IntoEnumIterator};
use tokio::time::{sleep, sleep_until, Sleep};
use types::{
    eth_spec::{EthSpec, MainnetEthSpec},
    Attestation, AttesterSlashing, BeaconBlockAltair, BeaconBlockBodyAltair, BeaconBlockBodyMerge,
    BeaconBlockMerge, ChainSpec, ExecutionPayload, FullPayload, Hash256, ProposerSlashing,
    Signature, SignedAggregateAndProof, SignedBeaconBlock, SignedBeaconBlockMerge,
    SignedContributionAndProof, SignedVoluntaryExit, Slot, SubnetId, SyncCommitteeMessage,
    SyncSubnetId,
};

#[cfg(test)]
mod tests;

#[derive(EnumIter, Debug, strum::Display, Clone, Copy)]
#[strum(serialize_all = "kebab_case")]
pub enum MsgType {
    BeaconBlock,
    AggregateAndProofAttestation,
    Attestation,
    VoluntaryExit,
    ProposerSlashing,
    AttesterSlashing,
    SignedContributionAndProof,
    SyncCommitteeMessage,
}

pub struct Generator<S, M> {
    slot_clock: S,
    slots_per_epoch: u64,
    validators: HashSet<u64>,
    subnets: u64,
    total_validators: u64,
    queued_messages: VecDeque<M>,
    next_slot: Pin<Box<Sleep>>,
}

type MT = Message;

pub type ValId = u64;

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum Message {
    BeaconBlock { proposer: ValId },
    AggregateAndProofAttestation { aggregator: ValId, committee: u64 },
    Attestation { attester: ValId, committee: u64 },
    VoluntaryExit,
    ProposerSlashing,
    AttesterSlashing,
    SignedContributionAndProof,
    SyncCommitteeMessage,
}

const TARGET_AGGREGATORS: u64 = 16;
const EPOCHS_PER_SYNC_COMMITTEE_PERIOD: u64 = 256;

impl<S: SlotClock> Generator<S, MT> {
    pub fn new(
        genesis_slot: Slot,
        genesis_duration: Duration,
        slots_per_epoch: u64,
        slot_duration: Duration,
        subnets: u64,
        validators: HashSet<u64>,
        total_validators: u64,
    ) -> Self {
        assert!(
            validators
                .iter()
                .max()
                .map(|max_val_id| max_val_id < &total_validators)
                .unwrap_or(true),
            "validator ids should go up to total_validators - 1"
        );
        assert!(
            total_validators >= subnets * TARGET_AGGREGATORS,
            "not enough validators to reach the target aggregators"
        );
        let slot_clock = S::new(genesis_slot, genesis_duration, slot_duration);
        let duration_to_next_slot = slot_clock
            .duration_to_next_slot()
            .expect("nothing ever goes wrong");
        Generator {
            slot_clock,
            slots_per_epoch,
            validators,
            subnets,
            total_validators,
            queued_messages: VecDeque::new(),
            next_slot: Box::pin(sleep(duration_to_next_slot)),
        }
    }

    pub fn get_msg(&self, current_slot: Slot, kind: MsgType) -> Vec<MT> {
        let slot = current_slot.as_u64();
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
                let epoch = current_slot.epoch(self.slots_per_epoch).as_u64();
                self.validators
                    .iter()
                    .filter_map(|val_id| {
                        // shake the val id using the epoch
                        let shaked_val_id = val_id.overflowing_add(epoch).0;
                        // assign to one of the committees
                        let committee = shaked_val_id % self.subnets;
                        // get an id on the range of existing validator ids and use it to get an id
                        // inside the committee
                        let idx_in_commitee =
                            (shaked_val_id % self.total_validators) / self.subnets;
                        let is_aggregator = idx_in_commitee / TARGET_AGGREGATORS == 0;
                        is_aggregator.then(|| Message::AggregateAndProofAttestation {
                            aggregator: *val_id,
                            committee,
                        })
                    })
                    .collect()
            }
            MsgType::Attestation => {
                let epoch = current_slot.epoch(self.slots_per_epoch).as_u64();
                self.validators
                    .iter()
                    .filter_map(|val_id| {
                        // shake the val id using the epoch
                        let shaked_val_id = val_id.overflowing_add(epoch).0;
                        // assign to one of the committees
                        let committee = shaked_val_id % self.subnets;
                        // assign attesters using the slot
                        let is_attester =
                            val_id.overflowing_add(slot).0 % self.slots_per_epoch == 0;
                        is_attester.then(|| Message::Attestation {
                            attester: *val_id,
                            committee,
                        })
                    })
                    .collect()
            }
            MsgType::VoluntaryExit | MsgType::ProposerSlashing | MsgType::AttesterSlashing => {
                // ignore them
                vec![]
            }
            // MsgType::SignedContributionAndProof => todo!(),
            MsgType::SyncCommitteeMessage => {
                // let epoch = current_slot.epoch(self.slots_per_epoch).as_u64();
                // let sync_committee_period = epoch / EPOCHS_PER_SYNC_COMMITTEE_PERIOD;
                self.validators
                    .iter()
                    .filter_map(|_val_id| {
                        // shake the val id using the sync_committee_period
                        // let shaked_val_id = val_id.overflowing_add(sync_committee_period).0;

                        Some(Message::SyncCommitteeMessage)
                    })
                    .collect()
            }
            _ => {
                let kind_: u64 = (kind as usize).try_into().unwrap();
                if current_slot % 8 == kind_ {
                    vec![Message::SignedContributionAndProof]
                } else {
                    vec![]
                }
            }
        }
    }

    fn queue_slot_msgs(&mut self, current_slot: Slot) {
        //     for msg_type in MsgType::iter() {
        //         if let Some(msg) = self.get_msg(current_slot, msg_type) {
        //             self.queued_messages.push_back(msg);
        //         }
        //     }
        //     tracing::info!(
        //         "[{current_slot}] Messages: len:{} {:?}",
        //         self.queued_messages.len(),
        //         self.queued_messages
        //     );
    }
}

// impl<S: SlotClock + Unpin> Stream for Generator<S, MT> {
//     type Item = MT;
//
//     fn poll_next(
//         mut self: Pin<&mut Self>,
//         cx: &mut std::task::Context<'_>,
//     ) -> Poll<Option<Self::Item>> {
//         // If there were any messages remaining from the current slot, return them.
//         if let Some(msg) = self.queued_messages.pop_front() {
//             return Poll::Ready(Some(msg));
//         }
//
//         if self.next_slot.as_mut().poll(cx).is_ready() {
//             let current_slot = self.slot_clock.now().unwrap();
//             self.queue_slot_msgs(current_slot);
//
//             let duration_to_next_slot = self.slot_clock.duration_to_next_slot().unwrap();
//             tracing::debug!("Time to next slot {duration_to_next_slot:?}");
//             self.next_slot = Box::pin(sleep(duration_to_next_slot));
//             // We either have messages to return or need to poll the sleep
//             cx.waker().wake_by_ref();
//         }
//
//         Poll::Pending
//     }
// }
