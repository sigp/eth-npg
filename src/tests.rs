use super::*;
use quickcheck_macros::quickcheck;
use rand::seq::SliceRandom;

#[quickcheck]
fn test_expected_message_counts(
    node_count: usize,
    test_slot: u64,
    slots_per_epoch: u64,
    total_validators: u64,
    attestation_subnets: u64,
) -> quickcheck::TestResult {
    let mut builder = Generator::builder();
    let params = match builder
        .slots_per_epoch(slots_per_epoch)
        .total_validators(total_validators)
        .attestation_subnets(attestation_subnets)
        .build_params()
    {
        Err(_) => return quickcheck::TestResult::discard(),
        Ok(params) => params,
    };

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            test_expected_message_counts_fn(node_count, Slot::new(test_slot), params)
        });

    quickcheck::TestResult::passed()
}

/// Verifies that the right amount of distinct messages are generated.
fn test_expected_message_counts_fn(
    node_count: usize,
    test_slot: Slot,
    params: builder::GeneratorParams,
) {
    // Get the relevant time parameters.
    let slots_per_epoch = params.slots_per_epoch();

    // Calculate the expected number of messages for each kind.
    let expected_blocks: usize = 1;
    let expected_aggregates: usize =
        (params.target_aggregators() * params.attestation_subnets()) as usize;
    let expected_attestations_per_epoch = params.total_validators() as usize;
    let expected_attestations_per_slot: usize =
        expected_attestations_per_epoch / params.slots_per_epoch() as usize;
    let expected_sync_committee_msgs =
        (params.sync_committee_subnets() * params.sync_subnet_size()) as usize;
    let expected_sync_aggregates =
        (params.sync_committee_subnets() * params.target_aggregators()) as usize;

    // Setup the network, giving the `node_count` nodes each a random number of validators from the
    // total pool.
    let nodes = setup_network(node_count, params);

    // Helper closure to improve readability.
    let get_msgs = |nodes: &[Generator], msg_type: MsgType, test_slot: Slot| {
        nodes
            .iter()
            .flat_map(|g| g.get_msg(test_slot, msg_type))
            .collect::<HashSet<_>>()
    };

    // Test the number of expected blocks.
    let blocks = get_msgs(&nodes, MsgType::BeaconBlock, test_slot);
    assert_eq!(blocks.len(), expected_blocks);

    // Test the number of expected aggregates per slot.
    let aggregates = get_msgs(&nodes, MsgType::AggregateAndProofAttestation, test_slot);
    assert_eq!(aggregates.len(), expected_aggregates);

    // Test the number of attestations per slot and epoch.
    let epoch = test_slot.epoch(slots_per_epoch);
    let slot_iter = epoch.slot_iter(slots_per_epoch);

    let mut total_attestations = HashSet::with_capacity(expected_attestations_per_epoch);
    for current_slot in slot_iter {
        let slot_attestations = get_msgs(&nodes, MsgType::Attestation, current_slot);
        let count_difference = slot_attestations
            .len()
            .abs_diff(expected_attestations_per_slot);
        assert!(count_difference <= 1);
        total_attestations.extend(slot_attestations.into_iter());
    }
    assert_eq!(total_attestations.len(), expected_attestations_per_epoch);

    // Test the number of sync committee messages per slot.
    let sync_committee_msgs = get_msgs(&nodes, MsgType::SyncCommitteeMessage, test_slot);
    assert_eq!(sync_committee_msgs.len(), expected_sync_committee_msgs);

    // Test the number of sync committee aggregates.
    let sync_committee_msgs = get_msgs(&nodes, MsgType::SignedContributionAndProof, test_slot);
    assert_eq!(sync_committee_msgs.len(), expected_sync_aggregates);
}

/// Creates a "network" consisting of `node_count` nodes with approximately
/// `total_validators`/`node_count` randomized validators.
fn setup_network(node_count: usize, generator_params: builder::GeneratorParams) -> Vec<Generator> {
    let mut nodes: Vec<Generator> = Vec::with_capacity(node_count);
    let total_validators = generator_params.total_validators();

    // Create the list of all validator ids and shuffle them.
    let mut all_validators = (0..total_validators).into_iter().collect::<Vec<_>>();
    all_validators.shuffle(&mut rand::thread_rng());
    let per_node_vals = (total_validators as usize) / node_count;
    let mut all_validators = all_validators.chunks(per_node_vals);

    let mut vals = 0;
    while let Some(node_validators) = all_validators.next() {
        // get the validators for this node.
        let node_validators = HashSet::from_iter(node_validators.iter().copied());
        vals += node_validators.len();

        nodes.push(
            generator_params
                .clone()
                .build(node_validators)
                .expect("right parameters"),
        );
    }
    assert_eq!(vals, total_validators as usize);
    nodes
}
