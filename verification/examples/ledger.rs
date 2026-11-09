// Two accounts, one sender (actor 0), integer units and increasing request IDs.
pub struct LedgerState {
    pub debit: u64,
    pub credit: u64,
    pub sequence: u64,
}

pub fn transfer(mut state: LedgerState, actor: u64, id: u64, amount: u64) -> LedgerState {
    if actor != 0
        || id <= state.sequence
        || amount == 0
        || amount > state.debit
        || amount > u64::MAX - state.credit
    {
        return state;
    }
    state.debit -= amount;
    state.credit += amount;
    state.sequence = id;
    state
}
