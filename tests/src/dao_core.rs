use super::*;
use ckb_testtool::{builtin::ALWAYS_SUCCESS, context::Context};
use ckb_tool::ckb_hash::Blake2bBuilder;
use ckb_tool::ckb_types::{bytes::Bytes, core::TransactionBuilder, packed::*, prelude::*};

const MAX_CYCLES: u64 = 10_000_000;
const BLAKE2B256_HASH_LEN: usize = 32; // Number of bytes for a Blake2b-256 hash.

fn calculate_instance_id(
    seed_cell_outpoint: &OutPoint,
    output_index: usize,
) -> [u8; BLAKE2B256_HASH_LEN] {
    let mut blake2b = Blake2bBuilder::new(BLAKE2B256_HASH_LEN)
        .personal(b"ckb-default-hash")
        .build();

    blake2b.update(&seed_cell_outpoint.tx_hash().raw_data());
    blake2b.update(&seed_cell_outpoint.index().raw_data());
    blake2b.update(&(output_index as u32).to_le_bytes());

    let mut hash: [u8; BLAKE2B256_HASH_LEN] = [0; BLAKE2B256_HASH_LEN];
    blake2b.finalize(&mut hash);

    hash
}

#[test]
fn test_can_create_vote() {
    // deploy contract
    let mut context = Context::default();
    context.set_capture_debug(true);
    let contract_bin: Bytes = Loader::default().load_binary("dao-core");
    let out_point = context.deploy_cell(contract_bin);

    // prepare scripts
    let out_point_always_success = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let out_point_sudt = context.deploy_cell(Loader::default().load_binary("sudt"));

    let lock_script = context
        .build_script(&out_point_always_success, Default::default())
        .expect("script");
    let lock_script_hash_owner: [u8; 32] = lock_script.calc_script_hash().unpack();
    let script_args: Bytes = lock_script_hash_owner.to_vec().into();

    let input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(1000u64.pack())
            .lock(lock_script.clone())
            .build(),
        Bytes::new(),
    );

    let dao_core_type_script = context
        .build_script(
            &out_point,
            Bytes::from(calculate_instance_id(&input_out_point, 0).to_vec()),
        )
        .expect("script");
    let dao_core_type_script_dep = CellDep::new_builder().out_point(out_point).build();

    let sudt_type_script = context
        .build_script(&out_point_sudt, script_args)
        .expect("script");
    let sudt_dep = CellDep::new_builder()
        .out_point(out_point_sudt.clone())
        .build();

    // prepare cells

    let voters = vec![
        (lock_script.clone(), 10u128),
        (lock_script.clone(), 10u128),
        (lock_script.clone(), 10u128),
    ];

    let input = CellInput::new_builder()
        .previous_output(input_out_point)
        .build();
    let mut outputs = vec![CellOutput::new_builder()
        .capacity(500u64.pack())
        .type_(Some(dao_core_type_script.clone()).pack())
        .build()];

    let mut core_output_data: Vec<u8> = [].to_vec();

    let mut token_code_hash = sudt_type_script.calc_script_hash().as_bytes().to_vec();

    core_output_data.append(&mut token_code_hash);
    let mut vote_title = String::from("Should Christmas last all year?");

    while vote_title.len() < 32 {
        vote_title += " ";
    }

    let tokens_to_distribute: u128 = voters.iter().map(|(_, amount)| *amount).sum::<u128>();

    core_output_data.append(&mut Bytes::from(vote_title).to_vec());

    let mut total_distributed_tokens = tokens_to_distribute.to_le_bytes().to_vec();
    core_output_data.append(&mut total_distributed_tokens);

    let mut is_voting_finished = [0u8; 1].to_vec();
    core_output_data.append(&mut is_voting_finished);

    let mut vote_result_option_type = [0u8; 1].to_vec();
    core_output_data.append(&mut vote_result_option_type);

    let mut outputs_data = vec![Bytes::from(core_output_data)];

    for (voter_lock_script, voter_sudt_amount) in voters {
        outputs.push(
            CellOutput::new_builder()
                .capacity(500u64.pack())
                .lock(voter_lock_script.clone())
                .type_(Some(sudt_type_script.clone()).pack())
                .build(),
        );
        outputs_data.push(Bytes::from(voter_sudt_amount.to_le_bytes().to_vec()));
    }

    println!("OUTPUTS DATA: {:?}", outputs_data);

    // build transaction
    let tx = TransactionBuilder::default()
        .input(input)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .cell_dep(dao_core_type_script_dep)
        .cell_dep(sudt_dep)
        .build();
    let tx = context.complete_tx(tx);

    // run
    context.verify_tx(&tx, MAX_CYCLES).unwrap();
    println!("DEBUG MESSAGES: {:?}", context.captured_messages());
}
