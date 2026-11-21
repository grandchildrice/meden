/** English presentation of the recording. Original source/tool bytes stay in snapshot.json. */
export const messages: Record<number, string> = {
  0: "Can you continue development?",
  1: "Yes. I reviewed the existing files.\n\nThe Rust vending machine already supports insertion, purchases, refunds, and Japanese/English commands.\n\nThe README records earlier formal-verification attempts, but **this session has no accepted contract or verified evidence**. Resuming verification requires proposing and reviewing a contract first.\n\nPossible next steps:\n- Regression tests for normal operations, boundaries and invalid inputs\n- Formal verification after reviewing the state-transition specification\n- Features such as multiple products, configurable prices or restocking",
  2: "Please resume formal verification.",
  3: "What happened?",
  4: "I submitted the existing vending-machine specification, but **Lean reached its maximum recursion depth and stopped**.\n\n- This does not mean a program bug or specification violation was found.\n- Verification is incomplete, so correctness has not been proved.\n- The contract precheck stopped before host review.\n- Source files were unchanged.\n\nThe README records similar errors from earlier attempts; this failure occurred before checking the Rust implementation.\n\nNext, I will reorganize the specification conditions without removing properties or narrowing the verification scope, then resubmit. I cannot change the verifier limits; if it still stops, I will report the tool limitation.",
  5: "Please continue",
  6: "I will preserve the specification and all properties, replacing whole-state branches with per-field expressions before resubmitting.",
  7: "**Formal verification succeeded (`verified: true`).**\n\n- Implemented operation branches and pre-addition boundary checks in `kernel.rs`.\n- The first implementation did not complete correspondence checking; after restructuring its branches, `verify_project` succeeded.\n- The accepted contract and verifier were unchanged.\n- Updated the README; `cargo build --offline` also passed.\n\nThe scope is the pure `step` for all u64 states and inputs. All eight accepted properties cover insertion, overflow refusal, purchase, full refund and state preservation on invalid operations.\n\nCLI parsing, rendering and I/O, and whole-application correctness are outside this proof.",
  8: "Can you continue?",
  9: "I will add CLI regression tests without changing verified `kernel.rs`. The tests cover parsing and display behavior outside the formal proof scope.",
  10: "**Added automated CLI regression tests.**\n\nThe seven tests in `tests/cli.rs` cover:\n- Japanese/English commands and whitespace\n- Successful purchases, insufficient balance and no stock\n- Full and repeated refunds\n- The u64 maximum and insertion overflow refusal\n- Invalid inputs and startup arguments\n- No automatic refund on exit or EOF\n\nResults:\n- `cargo test --offline`: all seven passed\n- `verify_project`: recheck passed (`verified: true`)\n- README updated with test instructions\n\n`kernel.rs` and the accepted contract are unchanged. Formal verification still covers only the pure `step`; CLI tests are separate behavioral checks.",
  12: "Hello! Would you like to continue the vending machine simulator, or work on something else?",
};
export const openingText =
  "Build a vending machine simulator in Rust with one product priced at 150 yen. Support inserting money, buying and refunds, and display balance and stock. Insufficient balance or no stock must leave the state unchanged. Only a successful purchase subtracts 150 yen and one item. Refund the full inserted balance and reset it to zero. Reject integer overflow. Formally verify the calculations as a pure Rust state transition, provide a standard-library-only CLI, and state the verification scope and unchecked parts.";
export const propertyDescriptions = [
  "A non-overflowing insertion adds exactly the amount, preserving stock and last refund.",
  "Insertion overflow preserves the entire state.",
  "With sufficient balance and stock, purchase subtracts exactly 150 and one item, preserving last refund.",
  "Insufficient balance or no stock preserves the entire state.",
  "Refund records the whole balance as the last refund, resets balance to zero and preserves stock.",
  "Unsupported operations preserve the entire state.",
  "Only successful purchases change stock.",
  "Only refunds change the last refunded amount.",
];
export const assumptions = [
  "All state fields, action and amount are u64 values. No other initial-state restriction applies.",
  "Verification covers only the pure step in kernel.rs. CLI parsing, rendering, initialization, I/O, persistence, concurrency, physical cash, the compiler and runtime environment are excluded.",
  "The product price is fixed at 150 yen. Only insertion uses amount. The refund field records the latest refund, not a cumulative total.",
];
