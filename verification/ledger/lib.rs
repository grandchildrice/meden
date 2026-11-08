pub mod ledger;

#[cfg(test)]
mod tests {
    use super::ledger::{transfer, LedgerState};

    #[test]
    fn accepted_contract_examples() {
        for (before, actor, id, amount, expected) in [
            ((10, 20, 0), 0, 1, 3, (7, 23, 1)),
            ((10, 20, 0), 1, 1, 3, (10, 20, 0)),
            ((10, 20, 0), 0, 1, 11, (10, 20, 0)),
            ((10, u64::MAX - 1, 0), 0, 1, 2, (10, u64::MAX - 1, 0)),
            ((10, 20, 0), 0, 0, 3, (10, 20, 0)),
            ((10, 20, 10), 0, 9, 3, (10, 20, 10)),
            ((10, 20, 0), 0, 1, 0, (10, 20, 0)),
            ((u64::MAX, 0, 0), 0, 1, u64::MAX, (0, u64::MAX, 1)),
            ((10, 20, u64::MAX - 1), 0, u64::MAX, 3, (7, 23, u64::MAX)),
        ] {
            eprintln!("checking before={before:?}, actor={actor}, id={id}, amount={amount}; expected={expected:?}");
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
                "before={before:?}, actor={actor}, id={id}, amount={amount}"
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
