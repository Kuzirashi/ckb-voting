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

- 1 DAO CORE CELL
- 2 DAO VOTE CELLS (YES/NO)
- X UDT CELLS TO ADDRESSES

X is the number of voters. The addresses of the voters should be known before creating the vote.

## DAO CORE CELL

### Type Script:

Args in Type Script should be blake2b256 hash of first Input Cell in transaction. This is so-called type ID pattern. It is generated based on Input Cell Outpoint + Output index of Core Cell: 0.

### Data:
- 32 bytes - VOTE_TITLE
- 8 bytes - TOTAL_DISTRIBUTED_TOKENS
- 1 byte - IS_VOTING_FINISHED
- 1 byte - VOTE_RESULT_OPTION_TYPE

VOTE_TITLE - Title of the vote
TOTAL_DISTRIBUTED_TOKENS - Total tokens distributed to all addresses
IS_VOTING_FINISHED - 0 = NO, 1 = YES
VOTE_RESULT_OPTION_TYPE - Result of the vote, 0 = NO, 1 = YES

## DAO VOTE CELL

### Type Script:

Args in Type Script should be blake2b256 hash of first Input Cell in transaction. This is so-called type ID pattern. It is generated based on Input Cell Outpoint + Output index of Core Cell: 1 (for NO cell) or 2 (for YES cell).

### Data:
- 1 byte - VOTE_OPTION_TYPE
- 8 bytes - TOTAL_VOTES_COLLECTED

VOTE_OPTION_TYPE - 0 for no, 1 for yes
TOTAL_VOTES_COLLECTED - all UDT tokens collected by this cell as votes

### Logic

1. Only 1 Cell of this type can exist in the output except for Minting and Burning the cell itself.
2. 2 Cells of this type need to be passed as input when Settling the vote.

# Transactions

# Create new vote

Assuming we have 3 voters.

Input:
1. A cell that's going to be used as a Seed Cell (could be any cell)

Output:
1. Core Cell
2. Vote No Cell
3. Vote Yes Cell
4. UDT Voter 1 Cell
5. UDT Voter 2 Cell
6. UDT Voter 3 Cell

# Vote

Assuming Voter 1 votes for No.

Input:

1. Vote No Cell
2. UDT Voter 1 Cell

Output:

1. Vote No Cell
2. UDT Voter 1 Cell if not all tokens were used for voting

# Finish voting

ONE OF THE VOTE CELLS NEED TO HAVE 51% OF ALL MINTED TOKENS.

Input:
1. Core Cell
2. Vote No Cell
3. Vote Yes Cell

Output:
1. Core Cell