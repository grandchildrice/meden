pub struct LedgerState {
    pub debit_balance: u64,
    pub credit_balance: u64,
    pub last_successful_request_id: u64,
}

pub fn transfer(s: LedgerState, actor: u64, request_id: u64, amount: u64) -> LedgerState {
    if actor == 0
        && amount > 0
        && request_id > s.last_successful_request_id
        && s.debit_balance >= amount
        && s.credit_balance <= u64::MAX - amount
    {
        LedgerState {
            debit_balance: s.debit_balance - amount,
            credit_balance: s.credit_balance + amount,
            last_successful_request_id: request_id,
        }
    } else {
        s
    }
}
