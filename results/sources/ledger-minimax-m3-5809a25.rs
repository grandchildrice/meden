pub struct LedgerState {
    pub debit_balance: u64,
    pub credit_balance: u64,
    pub last_request_id: u64,
}

pub fn transfer(s: LedgerState, actor: u64, request_id: u64, amount: u64) -> LedgerState {
    if actor == 0
        && request_id > s.last_request_id
        && amount > 0
        && s.debit_balance >= amount
        && s.credit_balance.checked_add(amount).is_some()
    {
        LedgerState {
            debit_balance: s.debit_balance - amount,
            credit_balance: s.credit_balance + amount,
            last_request_id: request_id,
        }
    } else {
        s
    }
}
