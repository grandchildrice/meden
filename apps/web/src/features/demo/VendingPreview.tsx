import { useState } from "react";
import { Alert, Button, TextField, Typography } from "@mui/material";
import LocalDrinkOutlined from "@mui/icons-material/LocalDrinkOutlined";

const MAX = (1n << 64n) - 1n;
export type Machine = { balance: bigint; stock: bigint; refunded: bigint };
/** UI preview of meden-demo/kernel.rs; no CLI/prover process or external effect. */
export function previewStep(
  state: Machine,
  action: "insert" | "buy" | "refund",
  amount = 0n,
): Machine {
  if (action === "insert")
    return amount >= 0n && amount <= MAX - state.balance
      ? { ...state, balance: state.balance + amount }
      : state;
  if (action === "buy")
    return state.balance >= 150n && state.stock > 0n
      ? { ...state, balance: state.balance - 150n, stock: state.stock - 1n }
      : state;
  return { ...state, balance: 0n, refunded: state.balance };
}
export function VendingPreview() {
  const [state, setState] = useState<Machine>({
    balance: 0n,
    stock: 10n,
    refunded: 0n,
  });
  const [amount, setAmount] = useState("150");
  const [notice, setNotice] = useState("");
  const [error, setError] = useState(false);
  const apply = (action: "insert" | "buy" | "refund") => {
    let value = 0n;
    if (action === "insert") {
      if (!/^\d+$/.test(amount)) {
        setNotice("Enter a nonnegative integer");
        setError(true);
        return;
      }
      value = BigInt(amount);
      if (value > MAX) {
        setNotice("Amount exceeds the u64 range");
        setError(true);
        return;
      }
    }
    const next = previewStep(state, action, value);
    setState(next);
    setError(next === state);
    setNotice(
      action === "insert"
        ? next === state
          ? "Rejected because the balance would overflow"
          : value.toLocaleString("en-US") + " yen inserted"
        : action === "buy"
          ? next === state
            ? state.stock === 0n
              ? "Out of stock"
              : "Insufficient balance"
            : "Purchased one item"
          : state.balance.toLocaleString("en-US") + " yen refunded",
    );
  };
  return (
    <div className="vending-preview">
      <div className="preview-heading">
        <Typography variant="h2">Vending machine simulator</Typography>
        <Typography color="text.secondary" variant="body2">
          meden-demo / kernel.rs
        </Typography>
      </div>
      <div className="vending-product">
        <div className="drink-icon">
          <LocalDrinkOutlined />
        </div>
        <Typography>Item</Typography>
        <strong>
          150<span> yen</span>
        </strong>
        <span className="stock-count">
          Stock {state.stock.toString()} items
        </span>
      </div>
      <div className="vending-balance">
        <span>Balance</span>
        <strong>
          {state.balance.toLocaleString("en-US")}
          <small> yen</small>
        </strong>
      </div>
      <div className="insert-row">
        <TextField
          size="small"
          label="Insert amount (yen)"
          value={amount}
          onChange={(event) => setAmount(event.target.value)}
          slotProps={{ htmlInput: { inputMode: "numeric", maxLength: 20 } }}
        />
        <Button variant="outlined" onClick={() => apply("insert")}>
          Insert
        </Button>
      </div>
      <div className="vending-actions">
        <Button variant="contained" onClick={() => apply("buy")}>
          Buy
        </Button>
        <Button variant="outlined" onClick={() => apply("refund")}>
          Refund
        </Button>
      </div>
      {notice && (
        <Alert severity={error ? "warning" : "success"}>{notice}</Alert>
      )}
      <Typography variant="caption" color="text.secondary">
        Simulates the pure step function in your browser. No real money or
        inventory is involved.
      </Typography>
      <Button
        size="small"
        color="secondary"
        onClick={() => {
          setState({ balance: 0n, stock: 10n, refunded: 0n });
          setNotice("");
        }}
      >
        Reset
      </Button>
    </div>
  );
}
