// Import from `core` instead of from `std` since we are in no-std mode.
use core::{ops::Add, result::Result};

use ckb_std::{ckb_constants::Source, ckb_types::packed::OutPoint, high_level::load_input};
use ckb_std::{ckb_types::bytes::Bytes, high_level::load_cell_type};
use ckb_std::{ckb_types::packed::Byte, high_level::load_script};
use ckb_std::{ckb_types::prelude::*, debug};

// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use ckb_std::high_level::load_cell_lock;
use ckb_std::high_level::{load_cell, load_cell_data, QueryIter};

use blake2b_ref::Blake2bBuilder;

// Import local modules.
use crate::error::Error;

// The modes of operation for the script.
enum Mode {
    Burn,
    Create,
    Transfer,
}

// Constants
const BLAKE2B256_HASH_BYTESIZE: usize = 32;
const CODE_HASH_BYTESIZE: usize = 32;
const U128_BYTESIZE: usize = 16;
const ARGS_BYTESIZE: usize = BLAKE2B256_HASH_BYTESIZE;

const VOTE_TITLE_BYTESIZE: usize = 32;
const TOTAL_DISTRIBUTED_TOKENS_BYTESIZE: usize = 16;
const IS_VOTING_FINISHED_BYTESIZE: usize = 1;
const VOTE_RESULT_OPTION_TYPE: usize = 1;
const DATA_LEN: usize = CODE_HASH_BYTESIZE
    + VOTE_TITLE_BYTESIZE
    + TOTAL_DISTRIBUTED_TOKENS_BYTESIZE
    + IS_VOTING_FINISHED_BYTESIZE
    + VOTE_RESULT_OPTION_TYPE; // Number of bytes required for args. (82 bytes)

// Determines the mode of operation for the currently executing script.
fn determine_mode() -> Result<Mode, Error> {
    // Gather counts on the number of group input and groupt output cells.
    let group_input_count = QueryIter::new(load_cell, Source::GroupInput).count();
    let group_output_count = QueryIter::new(load_cell, Source::GroupOutput).count();

    // Detect the operation based on the cell count.
    if group_input_count == 1 && group_output_count == 0 {
        return Ok(Mode::Burn);
    }
    if group_input_count == 0 && group_output_count == 1 {
        return Ok(Mode::Create);
    }
    if group_input_count == 1 && group_output_count == 1 {
        return Ok(Mode::Transfer);
    }

    // If no known code structure was used, return an error.
    Err(Error::InvalidTransactionStructure)
}

fn calculate_instance_id(
    seed_cell_outpoint: &OutPoint,
    output_index: usize,
) -> [u8; BLAKE2B256_HASH_BYTESIZE] {
    let mut blake2b = Blake2bBuilder::new(BLAKE2B256_HASH_BYTESIZE)
        .personal(b"ckb-default-hash")
        .build();

    blake2b.update(&seed_cell_outpoint.tx_hash().raw_data());
    blake2b.update(&seed_cell_outpoint.index().raw_data());
    blake2b.update(&(output_index as u32).to_le_bytes());

    // debug!("calc tx hash: {:?}", seed_cell_outpoint.tx_hash().raw_data());
    // debug!("calc index: {:?}", seed_cell_outpoint.index().raw_data());
    // debug!("calc output index: {:?}", (output_index as u32).to_le_bytes());

    let mut hash: [u8; BLAKE2B256_HASH_BYTESIZE] = [0; BLAKE2B256_HASH_BYTESIZE];
    blake2b.finalize(&mut hash);

    hash
}

// Validate a transaction to create a cell.
fn validate_create() -> Result<(), Error> {
    // Load the output cell data and verify that the value is 0u64.
    let cell_data = load_cell_data(0, Source::GroupOutput)?;
    let cell_type = load_cell_type(0, Source::GroupOutput)?.unwrap();
    let cell_type_args: Bytes = cell_type.args().unpack();
    let cell_lock = load_cell_lock(0, Source::GroupOutput)?;
    let cell_lock_args: Bytes = cell_lock.args().unpack();

    if cell_lock_args.len() != 0 {
        return Err(Error::LockArgsOtherThanZero);
    }

    // Determine the Seed Cell Outpoint.
    let seed_cell_outpoint = load_input(0, Source::Input)?.previous_output();

    let instance_id = calculate_instance_id(&seed_cell_outpoint, 0);
    // debug!("Output Instance ID: {:?}", output_nft_data.instance_id);
    // debug!("Calculated Instance ID: {:?}", instance_id);

    if cell_type_args.to_vec() != instance_id {
        return Err(Error::InvalidInstanceId);
    }

    if cell_data.len() != DATA_LEN {
        debug!("LEN IS: {:?}", cell_data.len());
        return Err(Error::InvalidDataBytesize);
    }

    // Token Code Hash
    let mut buffer = [0u8; 32];
    buffer.copy_from_slice(&cell_data[0..32]);
    let token_code_hash = Bytes::from(buffer.to_vec());

    // Vote title
    // let mut buffer = [0u8; 32];
    // buffer.copy_from_slice(&cell_data[32..64]);

    // Total distributed tokens
    let mut buffer = [0u8; 16];
    buffer.copy_from_slice(&cell_data[64..80]);
    let expected_total_tokens_distributed = u128::from_le_bytes(buffer);

    let mut tokens_distributed: u128 = 0;

    // Load each cell from the outputs.
    for (i, cell) in QueryIter::new(load_cell, Source::Output).enumerate() {
        // Check if there is a type script, and skip to the next cell if there is not.
        let cell_type_script = &cell.type_();

        if cell_type_script.is_none() {
            continue;
        }

        // Convert the scripts to bytes and check if they are the same.
        let cell_type_script = cell_type_script.to_opt().unwrap();
        let HASH_TYPE_DATA: Byte = 0.into();

        if cell_type_script.hash_type() == HASH_TYPE_DATA
            && *cell_type_script.code_hash().as_bytes() == *token_code_hash
        {
            let data = load_cell_data(i, Source::Output)?;

            let mut buffer = [0u8; 16];
            buffer.copy_from_slice(&cell_data[0..16]);
            let token_amount = u128::from_le_bytes(buffer);

            tokens_distributed.add(token_amount);
        }
    }

    debug!("Total tokens distributed: {:?}", tokens_distributed);

    if expected_total_tokens_distributed != tokens_distributed {
        return Err(Error::TokenDistributionMismatch);
    }

    Ok(())
}

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let args = script.args();

    if args.len() < ARGS_BYTESIZE {
        return Err(Error::InvalidArgsLength);
    }

    match determine_mode() {
        Ok(Mode::Burn) => return Ok(()),
        Ok(Mode::Create) => validate_create()?,
        Ok(Mode::Transfer) => return Err(Error::InvalidTransactionStructure),
        Err(e) => return Err(e),
    }

    Ok(())
}
