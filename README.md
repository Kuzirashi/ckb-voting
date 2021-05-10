# ckb-voting

Build contracts:

``` sh
capsule build
```

Run tests:

``` sh
capsule test
```

# Architecture

- 1 Core Cell
- 2 Vote Cells (yes/no)
- X SUDT Cells

X is the number of voters. The addresses of the voters should be known before creating the vote.

## Core Cell

### Type Script:

Args in Type Script should be blake2b256 hash of first Input Cell in transaction. This is so-called type ID pattern. It is generated based on Input Cell Outpoint + Output index of Core Cell: 0.

### Data

| Bytesize  | Name                     | Description
| --------- | ------                   | ----
| 32        | TOKEN_CODE_HASH          | Voting token code hash, hash_type is data
| 32        | VOTE_TITLE               | Title of the vote
| 16        | TOTAL_DISTRIBUTED_TOKENS | Total tokens distributed to all addresses
| 1         | IS_VOTING_FINISHED       | 0 = NO, 1 = YES
| 1         | VOTE_RESULT_OPTION_TYPE  | Result of the vote, 0 = NO, 1 = YES

## Vote Cell

### Type Script:

Vote Cell args should be exactly the same as Core Cell's args.

### Data:
- 1 byte - VOTE_OPTION_TYPE
- 8 bytes - TOTAL_VOTES_COLLECTED

VOTE_OPTION_TYPE - 0 for no, 1 for yes
TOTAL_VOTES_COLLECTED - all UDT tokens collected by this cell as votes

### Logic

1. Only 1 Cell of this type can exist in the output except for Minting and Burning the cell itself.
2. 2 Cells of this type need to be passed as input when Settling the vote.
3. If Vote Cell is used in conjunction with Core Cell in the same transaction Vote Cell args should be the same as Core Cell args.

## Simple User Defined Token (SUDT)

Token Type Script is SUDT. It is possible to mint, transfer and burn this token. 

Lock Script: Anyone Can Pay

### Type Script

- code_hash: sudt type script
- args: owner lock script hash

### Data

- amount: uint128 (16 bytes)

# Transactions

## Create new vote

[Check "test_can_create_vote" in dao_core.rs.](tests/src/dao_core.rs)

Assuming we have 3 voters.

Input:
1. A cell that's going to be used as a Seed Cell (could be any cell)

Output:
1. Core Cell
2. Vote No Cell
3. Vote Yes Cell
4. SUDT Voter 1 Cell
5. SUDT Voter 2 Cell
6. SUDT Voter 3 Cell

Voter Cells with SUDT are locked with ACP locks for the addresses. 

## Vote

Assuming Voter 1 votes for No.

Input:

1. Vote No Cell
2. SUDT Voter 1 Cell

Output:

1. Vote No Cell
2. SUDT Voter 1 Cell if not all tokens were used for voting

## Finish voting

ONE OF THE VOTE CELLS NEED TO HAVE 51% OF ALL MINTED TOKENS.

Input:
1. Core Cell
2. Vote No Cell
3. Vote Yes Cell

Output:
1. Core Cell

# Known issues

We're using a Simple User Defined Token standard and we're not restricting how it could be minted. We could do so but we chose not to for the sake of simplicity.

To detect a fraud in this simple application we could scan the chain for the total number of SUDT in circulation and if we see that the number went up for no good reason, we could withdraw from honoring vote result.

We're designing a simple voting system. This would be sufficient for on-chain voting, but off-chain execution. For example, we vote on a new chairman for the committee. The blockchain is evidence of the vote, but the actual handing is done in real life, not attached to the chain at all. In this respect, being able to detect a fraud by organizers is all that is important. However, this type of a system is not sufficient for something like on-chain management of a large sum of cryptocurrency with automatic execution of transfers based on voting. 
