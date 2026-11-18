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
        setNotice("0以上の整数を入力してください");
        setError(true);
        return;
      }
      value = BigInt(amount);
      if (value > MAX) {
        setNotice("金額がu64の範囲を超えています");
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
          ? "残高がオーバーフローするため拒否しました"
          : value.toLocaleString("ja-JP") + "円を投入しました"
        : action === "buy"
          ? next === state
            ? state.stock === 0n
              ? "在庫切れです"
              : "残高が不足しています"
            : "商品を1個購入しました"
          : state.balance.toLocaleString("ja-JP") + "円を返金しました",
    );
  };
  return (
    <div className="vending-preview">
      <div className="preview-heading">
        <Typography variant="h2">自販機シミュレーター</Typography>
        <Typography color="text.secondary" variant="body2">
          meden-demo / kernel.rs
        </Typography>
      </div>
      <div className="vending-product">
        <div className="drink-icon">
          <LocalDrinkOutlined />
        </div>
        <Typography>商品</Typography>
        <strong>
          150<span>円</span>
        </strong>
        <span className="stock-count">在庫 {state.stock.toString()}個</span>
      </div>
      <div className="vending-balance">
        <span>残高</span>
        <strong>
          {state.balance.toLocaleString("ja-JP")}
          <small>円</small>
        </strong>
      </div>
      <div className="insert-row">
        <TextField
          size="small"
          label="投入金額（円）"
          value={amount}
          onChange={(event) => setAmount(event.target.value)}
          slotProps={{ htmlInput: { inputMode: "numeric", maxLength: 20 } }}
        />
        <Button variant="outlined" onClick={() => apply("insert")}>
          投入
        </Button>
      </div>
      <div className="vending-actions">
        <Button variant="contained" onClick={() => apply("buy")}>
          購入
        </Button>
        <Button variant="outlined" onClick={() => apply("refund")}>
          返金
        </Button>
      </div>
      {notice && (
        <Alert severity={error ? "warning" : "success"}>{notice}</Alert>
      )}
      <Typography variant="caption" color="text.secondary">
        純粋なstepの動きをブラウザで再現しています。実際の現金や在庫は操作しません。
      </Typography>
      <Button
        size="small"
        color="secondary"
        onClick={() => {
          setState({ balance: 0n, stock: 10n, refunded: 0n });
          setNotice("");
        }}
      >
        リセット
      </Button>
    </div>
  );
}
