// Import from `core` instead of from `std` since we are in no-std mode.
use core::result::Result;

use ckb_std::{ckb_types::prelude::*, debug};
use ckb_std::high_level::load_script;
use ckb_std::syscalls::load_witness;
use ckb_std::{ckb_constants::Source, ckb_types::packed::OutPoint, high_level::load_input};
use ckb_std::{ckb_types::bytes::Bytes, high_level::load_cell_type};

// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use ckb_std::high_level::load_cell_lock;
use ckb_std::high_level::{load_cell, load_cell_data, QueryIter};

use blake2b_ref::Blake2bBuilder;

// Import local modules.
use crate::error::Error;

// The modes of operation for the script.
enum Mode {
    Burn,     // Consume an existing counter cell.
    Create,   // Create a new counter cell.
    Transfer, // Transfer (update) a counter cell and increase its value.
}

// Constants
const BLAKE2B256_HASH_LEN: usize = 32; // Number of bytes for a Blake2b-256 hash.
const CKBDL_CONTEXT_SIZE: usize = 64 * 1024;
const CODE_HASH_NULL: [u8; 32] = [0u8; 32];
const U128_LEN: usize = 16; // Number of bytes for a 128-bit unsigned integer.
const INSTANCE_ID_LEN: usize = BLAKE2B256_HASH_LEN; // Number of bytes in the Instance ID field.
const LOCK_HASH_LEN: usize = BLAKE2B256_HASH_LEN; // Number of bytes for a lock hash. (Blake2b 32 bytes)
const QUANTITY_LEN: usize = U128_LEN; // Number of bytes in the quantity field.
const TOKEN_LOGIC_FUNCTION: &[u8] = b"token_logic";
const TOKEN_LOGIC_LEN: usize = BLAKE2B256_HASH_LEN; // Number of bytes in a Token Logic field.
const ARGS_LEN: usize = LOCK_HASH_LEN; // Number of bytes required for args. (32 bytes)

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

    //
    //  First 32 bytes = Vote name
    //
    if cell_data.len() != 32 {
        debug!("LEN IS: {:?}", cell_data.len());
        return Err(Error::InvalidOutputCellData);
    }

    // debug!("Calculated Instance ID: {:?}", instance_id);

    // let mut buffer = [0u8; 32];
    // buffer.copy_from_slice(&cell_data[0..32]);
    // let input_value = u64::from_le_bytes(buffer);

    // strin

    Ok(())
}

// Validate a transaction to transfer (update) a counter cell and increase its value.
fn validate_transfer() -> Result<(), Error> {
    // Load the input cell data and verify that the length is exactly 8, which is the length of a u64.
    let input_data = load_cell_data(0, Source::GroupInput)?;
    if input_data.len() != 8 {
        return Err(Error::InvalidInputCellData);
    }

    // Load the output cell data and verify that the length is exactly 8, which is the length of a u64.
    let output_data = load_cell_data(0, Source::GroupOutput)?;
    if output_data.len() != 8 {
        return Err(Error::InvalidOutputCellData);
    }

    // Create a buffer to use for parsing cell data into integers.
    let mut buffer = [0u8; 8];

    // Convert the input cell data to a u64 value.
    buffer.copy_from_slice(&input_data[0..8]);
    let input_value = u64::from_le_bytes(buffer);

    // Convert the output cell data to a u64 value.
    buffer.copy_from_slice(&output_data[0..8]);
    let output_value = u64::from_le_bytes(buffer);

    // Check for an overflow scenario.
    if input_value == u64::MAX {
        return Err(Error::CounterValueOverflow);
    }

    // Ensure that the output value is always exactly one more that in the input value.
    if input_value + 1 != output_value {
        return Err(Error::InvalidCounterValue);
    }

    Ok(())
}

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let args = script.args();

    if args.len() < ARGS_LEN {
        return Err(Error::InvalidArgsLen);
    }

    // Determine the mode and validate as needed.
    match determine_mode() {
        Ok(Mode::Burn) => return Ok(()),
        Ok(Mode::Create) => validate_create()?,
        Ok(Mode::Transfer) => validate_transfer()?,
        Err(e) => return Err(e),
    }

    Ok(())
}
