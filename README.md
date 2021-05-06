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

Voting:
- USER SENDS HIS UDT CELL AS INPUT
- AS OUTPUT THERE'S ONE OF VOTE CELLS WITH INCREASED VALUE
- TOKEN HAS TO BE BURNED IN THIS TRANSACTION

Collection:
- 2 VOTE CELLS SHOULD BE PASSED AS INPUT
- ONE OF THE VOTE CELLS NEED TO HAVE 51% OF ALL MINTED TOKENS

## DAO CORE CELL

### Type Script:

Args in Type Script should be blake2b256 hash of first Input Cell in transaction. This is so-called type ID pattern. It is generated based on Input Cell Outpoint + Output index of Core Cell: 0.

### Data:
- 32 bytes - VOTE_TITLE
- 8 bytes - TOTAL_DISTRIBUTED_TOKENS

VOTE_TITLE - Title of the vote
TOTAL_DISTRIBUTED_TOKENS - Total tokens distributed to all addresses

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