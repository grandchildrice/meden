import { useState } from "react";
import { IconButton, Popover, Typography } from "@mui/material";
import InfoOutlined from "@mui/icons-material/InfoOutlined";
import CloseRounded from "@mui/icons-material/CloseRounded";
const method = [
  [
    "定理を文章に戻す",
    "claimcheckの方式では、定理から自然文を作り直します。この段階では元の依頼文を見せず、依頼に都合よく訳されるのを避けます。",
  ],
  [
    "依頼と食い違っていないか調べる",
    "別のモデルで、元の依頼と作り直した文章を比較します。条件の抜け、余分な前提、数や対象の違いを確認します。",
  ],
  [
    "違う箇所を示す",
    "たとえば依頼が「150円あれば購入できる」でも、定理が「在庫がある場合だけ」としていれば、その条件を差分で示します。",
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
          公開されている意味の評価ツールを再利用する方針です。
        </Typography>
        <ol>
          {method.map(([title, text]) => (
            <li key={title}>
              <strong>{title}</strong>
              <p>{text}</p>
            </li>
          ))}
        </ol>
        <Typography variant="body2" color="text.secondary">
          候補はclaimcheckとFormalAlign。FormalAlignは、自然文とLeanの文が対応しているかを専用モデルで評価します。
        </Typography>
        <div className="semantic-method-links">
          <a
            href="https://github.com/metareflection/claimcheck"
            target="_blank"
            rel="noreferrer"
          >
            claimcheck
          </a>
          <a
            href="https://github.com/rookie-joe/FormalAlign"
            target="_blank"
            rel="noreferrer"
          >
            FormalAlign
          </a>
        </div>
      </Popover>
    </>
  );
}
