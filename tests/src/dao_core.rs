use super::*;
use ckb_testtool::{builtin::ALWAYS_SUCCESS, context::Context};
use ckb_tool::ckb_script::ScriptError;
use ckb_tool::ckb_types::{bytes::Bytes, core::TransactionBuilder, packed::*, prelude::*};
use ckb_tool::{ckb_error::assert_error_eq, ckb_hash::Blake2bBuilder};

const MAX_CYCLES: u64 = 10_000_000;
const BLAKE2B256_HASH_LEN: usize = 32; // Number of bytes for a Blake2b-256 hash.

// error numbers
const ERROR_NOT_EMPTY_LOCK_SCRIPT: i8 = 13;

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

    // debug!("calc tx hash: {:?}", seed_cell_outpoint.tx_hash().raw_data());
    // debug!("calc index: {:?}", seed_cell_outpoint.index().raw_data());
    // debug!("calc output index: {:?}", (output_index as u32).to_le_bytes());

    let mut hash: [u8; BLAKE2B256_HASH_LEN] = [0; BLAKE2B256_HASH_LEN];
    blake2b.finalize(&mut hash);

    hash
}

// 1. You create a vote eg. "Should we make Christmas last all year?" and pass a list of X addresses that should be able to vote on a proposal
// 2. Each of these X addresses get 10 vote-specific-UDT
// 3. 2 cells are created for YES/NO options
// 4. People have to send their UDT to one or the other cell
// 5. Whichever cell has more UDT this option wins

// I think the scope is only a bit bigger than what we currently cover in Developer Training Course and if we want the scope of the book to be broader - for example by introducing oracles - we could make it a prediction market easily.

// To make it a prediction market we would need to maintain in Vote Deposit Cells a mapping how many tokens were sent by each address and then based on that we could distribute some rewards after checking the oracle data cell.

// #[test]
// fn test_success() {
//     // deploy contract
//     let mut context = Context::default();
//     let contract_bin: Bytes = Loader::default().load_binary("dao-core");
//     let out_point = context.deploy_cell(contract_bin);

//     // prepare scripts
//     let lock_script = context
//         .build_script(&out_point, Bytes::from(vec![42]))
//         .expect("script");
//     let lock_script_dep = CellDep::new_builder()
//         .out_point(out_point)
//         .build();

//     // prepare cells
//     let input_out_point = context.create_cell(
//         CellOutput::new_builder()
//             .capacity(1000u64.pack())
//             .lock(lock_script.clone())
//             .build(),
//         Bytes::new(),
//     );
//     let input = CellInput::new_builder()
//         .previous_output(input_out_point)
//         .build();
//     let outputs = vec![
//         CellOutput::new_builder()
//             .capacity(500u64.pack())
//             .lock(lock_script.clone())
//             .build(),
//         CellOutput::new_builder()
//             .capacity(500u64.pack())
//             .lock(lock_script)
//             .build(),
//     ];

//     let outputs_data = vec![Bytes::new(); 2];

//     // build transaction
//     let tx = TransactionBuilder::default()
//         .input(input)
//         .outputs(outputs)
//         .outputs_data(outputs_data.pack())
//         .cell_dep(lock_script_dep)
//         .build();
//     let tx = context.complete_tx(tx);

//     // run
//     let cycles = context
//         .verify_tx(&tx, MAX_CYCLES)
//         .expect("pass verification");
//     println!("consume cycles: {}", cycles);
// }

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

    // let input_out_point = context.create_cell(CellOutput::new_builder().capacity(100_000_000_000_u64.pack()).lock(lock_script.clone()).build(), Bytes::new());
    // let input = CellInput::new_builder().previous_output(input_out_point).build();
    // inputs.push(input);

    let input = CellInput::new_builder()
        .previous_output(input_out_point)
        .build();
    let outputs = vec![
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .type_(Some(dao_core_type_script.clone()).pack())
            .build(),
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .type_(Some(sudt_type_script.clone()).pack())
            .build(),
    ];

    let mut output_data: Vec<u8> = [].to_vec();

    let mut token_code_hash = sudt_type_script.calc_script_hash().as_bytes().to_vec();

    // let mut token_code_hash = [0u8; 32].to_vec();
    output_data.append(&mut token_code_hash);
    let mut vote_title = String::from("Should Christmas last all year?");

    while vote_title.len() < 32 {
        vote_title += " ";
    }

    let tokens_to_distribute = 9_000u128;

    output_data.append(&mut Bytes::from(vote_title).to_vec());

    let mut total_distributed_tokens = tokens_to_distribute.to_le_bytes().to_vec();
    output_data.append(&mut total_distributed_tokens);

    let mut is_voting_finished = [0u8; 1].to_vec();
    output_data.append(&mut is_voting_finished);

    let mut vote_result_option_type = [0u8; 1].to_vec();
    output_data.append(&mut vote_result_option_type);

    let sudt_data = tokens_to_distribute.to_le_bytes().to_vec();

    let outputs_data = vec![Bytes::from(output_data), Bytes::from(sudt_data)];

    println!("OUTPUTS DATA: {:?}", outputs_data);

    // build transaction
    let tx = TransactionBuilder::default()
        .input(input)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .cell_dep(dao_core_type_script_dep)
        .build();
    let tx = context.complete_tx(tx);

    // run
    let result = context.verify_tx(&tx, MAX_CYCLES).unwrap();
    println!("DEBUG MESSAGES: {:?}", context.captured_messages());

    println!("DAO Core Cell created successfully.");

    // println!("{:?}", err);
    // we expect an error raised from 0-indexed cell's lock script
    // let dao_core_output_cell_index = 0;
    // assert_error_eq!(
    //     err,
    //     ScriptError::ValidationFailure(ERROR_NOT_EMPTY_LOCK_SCRIPT)
    //         .output_type_script(dao_core_output_cell_index)
    // );
}
