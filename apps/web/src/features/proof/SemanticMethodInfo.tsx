import { useState } from "react";
import { IconButton, Popover, Typography } from "@mui/material";
import InfoOutlined from "@mui/icons-material/InfoOutlined";
import CloseRounded from "@mui/icons-material/CloseRounded";
const method = [
  [
    "意味に関わる依存を抽出",
    "Lean Atlas / Compassで、対象定理の型・仮定・参照する定義を辿り、比較する範囲を絞ります。",
  ],
  [
    "形式的な構造を検査",
    "型、前提条件、量化、対象範囲を確認し、矛盾した仮定や空虚に成立する命題を調べます。",
  ],
  [
    "定理を逆翻訳",
    "元の要件を見せずに、Leanの宣言と依存する定義から自然言語の意味を復元します。コメントだけを根拠にしません。",
  ],
  [
    "独立した判定で比較",
    "別モデル・別プロンプトで元の要件と逆翻訳を比較し、条件・例外・量化の差を特定します。差分と境界ケースを根拠として示します。",
  ],
];
export function SemanticMethodInfo() {
  const [anchor, setAnchor] = useState<HTMLElement | null>(null);
  return (
    <>
      <IconButton
        aria-label="意味解析の仕組み"
        aria-haspopup="dialog"
        aria-expanded={Boolean(anchor)}
        onClick={(event) => setAnchor(event.currentTarget)}
        className="semantic-info-button"
      >
        <InfoOutlined fontSize="small" />
      </IconButton>
      <Popover
        open={Boolean(anchor)}
        anchorEl={anchor}
        onClose={() => setAnchor(null)}
        anchorOrigin={{ vertical: "bottom", horizontal: "right" }}
        transformOrigin={{ vertical: "top", horizontal: "right" }}
        slotProps={{
          paper: {
            role: "dialog",
            "aria-labelledby": "semantic-method-title",
            className: "semantic-method-info",
          },
        }}
      >
        <header>
          <Typography variant="h3" id="semantic-method-title">
            意味解析のアルゴリズム
          </Typography>
          <IconButton aria-label="説明を閉じる" onClick={() => setAnchor(null)}>
            <CloseRounded fontSize="small" />
          </IconButton>
        </header>
        <Typography variant="caption" color="text.secondary">
          採用予定の解析フロー
        </Typography>
        <ol>
          {method.map(([title, text]) => (
            <li key={title}>
              <strong>{title}</strong>
              <p>{text}</p>
            </li>
          ))}
        </ol>
        <Typography variant="caption" color="text.secondary">
          docs2formalspecの逆翻訳・独立Judgeと、Lean
          Atlasの依存解析を組み合わせる設計です。
        </Typography>
      </Popover>
    </>
  );
}
