pub struct FeeQuote {
    pub units: u64,
    pub remainder: u64,
}

pub fn fee_quote(s: FeeQuote, amount: u64) -> FeeQuote {
    if amount <= 737869762948382064 {
        let scaled = amount * 25;
        FeeQuote {
            units: scaled / 10000,
            remainder: scaled % 10000,
        }
    } else {
        s
    }
}
