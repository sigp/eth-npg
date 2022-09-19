use std::{
    collections::VecDeque, convert::TryInto, marker::PhantomData, pin::Pin, task::Poll,
    time::Duration,
};

use futures::{stream::Stream, Future, FutureExt};
use slot_clock::{SlotClock, SystemTimeSlotClock};
use strum::{EnumIter, IntoEnumIterator};
use tokio::time::{sleep, sleep_until, Sleep};
use types::{
    eth_spec::{EthSpec, MainnetEthSpec},
    ChainSpec, Slot,
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
    node_id: u64,
    total_nodes: u64,
    queued_messages: VecDeque<M>,
    next_slot: Pin<Box<Sleep>>,
}

// TODO: relation between EthSpec and SlotClock?
impl<S: SlotClock> Generator<S, String> {
    pub fn new(
        genesis_slot: Slot,
        genesis_duration: Duration,
        slot_duration: Duration,
        node_id: u64,
        total_nodes: u64,
    ) -> Self {
        assert!(
            node_id < total_nodes,
            "nodes ids should go up to total_nodes - 1"
        );
        println!("creating slot clock");
        let slot_clock = S::new(genesis_slot, genesis_duration, slot_duration);
        println!("slot clock created");
        let duration_to_next_slot = slot_clock
            .duration_to_next_slot()
            .expect("nothing ever goes wrong");
        println!("time to next slot: {duration_to_next_slot:?}");
        Generator {
            slot_clock,
            node_id,
            total_nodes,
            queued_messages: VecDeque::new(),
            next_slot: Box::pin(sleep(duration_to_next_slot)),
        }
    }

    // self? any state needed to get correct msg topic distribution?
    pub fn get_msg(&self, current_slot: Slot, kind: MsgType) -> Option<String> {
        match kind {
            MsgType::BeaconBlock => {
                return (current_slot % self.total_nodes == self.node_id)
                    .then(|| format!("{kind}, {current_slot}"))
            }
            // MsgType::AggregateAndProofAttestation => todo!(),
            // MsgType::Attestation => todo!(),
            // MsgType::VoluntaryExit => todo!(),
            // MsgType::ProposerSlashing => todo!(),
            // MsgType::AttesterSlashing => todo!(),
            // MsgType::SignedContributionAndProof => todo!(),
            // MsgType::SyncCommitteeMessage => todo!(),
            _ => {
                let kind_: u64 = (kind as usize).try_into().unwrap();
                if current_slot % 8 == kind_ {
                    Some(format!("{kind}, {}", current_slot.as_u64()))
                } else {
                    None
                }
            }
        }
    }

    fn queue_slot_msgs(&mut self, current_slot: Slot) {
        let mut msg_queued = false;
        for msg_type in MsgType::iter() {
            if let Some(msg) = self.get_msg(current_slot, msg_type) {
                tracing::info!("[{current_slot}] Queueing messsage: {msg}");
                self.queued_messages.push_back(msg);
                msg_queued = true;
            }
        }
        if !msg_queued {
            tracing::info!("No message for current slot {current_slot}");
        }
    }
}

impl<S: SlotClock + Unpin> Stream for Generator<S, String> {
    type Item = String;

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
            tracing::debug!("Time to next slot {duration_to_next_slot:?}");
            self.next_slot = Box::pin(sleep(duration_to_next_slot));
            // We either have messages to return or need to poll the sleep
            cx.waker().wake_by_ref();
        }

        Poll::Pending
    }
}
