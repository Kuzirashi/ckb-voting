// Import from core instead of from std since we are in no-std mode.
use core::result::Result;

use blake2b_ref::Blake2bBuilder;
// Import CKB syscalls and structures.
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use ckb_std::{ckb_constants::Source, ckb_types::packed::OutPoint, high_level::{load_cell, load_cell_type, load_input}};
use ckb_std::ckb_types::{bytes::Bytes, prelude::*};
use ckb_std::high_level::{load_script, load_cell_lock_hash, load_cell_data, QueryIter};

// Import our local error codes.
use crate::error::Error;

// Constants
const LOCK_HASH_LEN: usize = 32; // Number of bytes for a lock hash. (Blake2b 256-bit 32 bytes)
const SUDT_DATA_LEN: usize = 16; // SUDT uses a u128, which is 16 bytes.
const BLAKE2B256_HASH_LEN: usize = 32; // Number of bytes for a Blake2b-256 hash.

// Determines the mode of operation for the currently executing script.
fn is_mint_mode() -> bool {
    // Gather counts on the number of group input and groupt output cells.
    let group_input_count = QueryIter::new(load_cell, Source::GroupInput).count();
    let group_output_count = QueryIter::new(load_cell, Source::GroupOutput).count();

    // Detect the operation based on the cell count.
    if group_input_count == 0 && group_output_count > 0 {
        return true;
    }

    return false;
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
	let cell_type = load_cell_type(0, Source::GroupOutput)?.unwrap();
    let cell_type_args: Bytes = cell_type.args().unpack();

	 let seed_cell_outpoint = load_input(0, Source::Input)?.previous_output();

	 let instance_id = calculate_instance_id(&seed_cell_outpoint, 0);
 
	 if cell_type_args.to_vec() != instance_id {
		 return Err(Error::InvalidInstanceId);
	 }

    Ok(())
}

/// Count the number of tokens in the specified source. Source should be either GroupInput or GroupOutput.
fn determine_token_amount(source: Source) -> Result<u128, Error>
{
	// Track the number of tokens that are counted.
	let mut total_token_amount = 0;

	// Cycle through the data in each cell within the specified source.
	let cell_data = QueryIter::new(load_cell_data, source);
	for data in cell_data
	{
		// Check that the length of the data is >= 16 bytes, the size of a u128.
		if data.len() >= SUDT_DATA_LEN
		{
			// Convert the binary data in the cell to a u128 value.
			let mut buffer = [0u8; SUDT_DATA_LEN];
			buffer.copy_from_slice(&data[0..SUDT_DATA_LEN]);
			let amount = u128::from_le_bytes(buffer);

			// Add the amount of tokens in the cell to the total amount of tokens.
			total_token_amount += amount;
		}
		// If the data is less than 16 bytes, then return an encoding error.
		else
		{
			return Err(Error::Encoding);
		}
	}

	// Return the total amount of tokens found in the specified source.
	Ok(total_token_amount)
}


// Main entry point.
pub fn main() -> Result<(), Error>
{
	// Load the currently executing script and get the args.
	let script = load_script()?;
	let args: Bytes = script.args().unpack();

	if is_mint_mode() {
		return validate_create();
	}

	// Count the number of tokens in the GroupInput and GroupOutput.
	let input_token_amount = determine_token_amount(Source::GroupInput)?;
	let output_token_amount = determine_token_amount(Source::GroupOutput)?;

	// If the amount of input tokens is less than the amount of output tokens, return an error.   
	if input_token_amount < output_token_amount
	{
		return Err(Error::Amount);
	}

	// No errors were found during validation. Return success.
	Ok(())
}
