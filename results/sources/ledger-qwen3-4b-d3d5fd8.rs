pub struct LedgerState { pub debit: u64, pub credit: u64, pub sequence: u64 }

pub fn transfer(mut state: LedgerState, actor: u64, id: u64, amount: u64) -> LedgerState {
    // Only actor 0 is allowed to transfer
    if actor != 0 {
        return state;
    }

    // Request ID must be strictly greater than the last successful ID
    if id <= state.sequence {
        return state;
    }

    // Check for zero amount
    if amount == 0 {
        return state;
    }

    // Check if we have sufficient debit balance
    if state.debit < amount {
        return state;
    }

    // Check for overflow in credit before performing the addition
    // We need to ensure that state.credit + amount won't overflow
    // For u64, we can check if adding amount would exceed the maximum value
    if state.credit > u64::MAX - amount {
        return state;
    }

    // Perform the transfer
    let new_debit = state.debit - amount;
    let new_credit = state.credit + amount;
    let new_sequence = id;

    LedgerState {
        debit: new_debit,
        credit: new_credit,
        sequence: new_sequence,
    }
}