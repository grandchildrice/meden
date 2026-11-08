pub mod ledger;

#[cfg(test)]
mod tests {
    use super::ledger::{transfer, LedgerState};

    #[test]
    fn accepted_contract_examples() {
        for (requirement, before, actor, id, amount, expected) in [
            ("valid transfer moves the exact amount", (10, 20, 0), 0, 1, 3, (7, 23, 1)),
            ("valid request IDs may skip numbers", (10, 20, 0), 0, 7, 3, (7, 23, 7)),
            ("unauthorized actor leaves all state unchanged", (10, 20, 0), 1, 1, 3, (10, 20, 0)),
            ("insufficient debit leaves all state unchanged", (10, 20, 0), 0, 1, 11, (10, 20, 0)),
            ("credit overflow leaves all state unchanged", (10, u64::MAX - 1, 0), 0, 1, 2, (10, u64::MAX - 1, 0)),
            ("non-increasing request ID leaves all state unchanged", (10, 20, 0), 0, 0, 3, (10, 20, 0)),
            ("old request ID leaves all state unchanged", (10, 20, 10), 0, 9, 3, (10, 20, 10)),
            ("zero amount leaves all state unchanged", (10, 20, 0), 0, 1, 0, (10, 20, 0)),
            ("exact maximum credit is allowed", (u64::MAX, 0, 0), 0, 1, u64::MAX, (0, u64::MAX, 1)),
            ("maximum request ID is allowed", (10, 20, u64::MAX - 1), 0, u64::MAX, 3, (7, 23, u64::MAX)),
        ] {
            eprintln!("contract requirement: {requirement}; before={before:?}, actor={actor}, id={id}, amount={amount}; expected={expected:?}");
            let out = transfer(
                LedgerState {
                    debit: before.0,
                    credit: before.1,
                    sequence: before.2,
                },
                actor,
                id,
                amount,
            );
            assert_eq!(
                (out.debit, out.credit, out.sequence),
                expected,
                "contract requirement: {requirement}; before={before:?}, actor={actor}, id={id}, amount={amount}"
            );
        }
        let s = LedgerState {
            debit: 10,
            credit: 20,
            sequence: 0,
        };
        let s = transfer(s, 0, 1, 2);
        let s = transfer(s, 0, 2, 1);
        let s = transfer(s, 0, 1, 2);
        assert_eq!(
            (s.debit, s.credit, s.sequence),
            (7, 23, 2),
            "retry after intervening transfer"
        );
    }
}
