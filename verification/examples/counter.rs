pub struct Counter { pub value: u64 }
pub fn increment(s: Counter, amount: u64) -> Counter {
    match s.value.checked_add(amount) {
        Some(value) => Counter { value },
        None => s,
    }
}
