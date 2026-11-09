pub struct FeeQuote { pub units: u64, pub remainder: u64 }
pub fn quote(s: FeeQuote, amount: u64) -> FeeQuote {
    if amount > u64::MAX / 25 { return s; }
    let scaled = amount * 25;
    FeeQuote { units: scaled / 10000, remainder: scaled % 10000 }
}
