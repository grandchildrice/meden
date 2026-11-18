import { useState } from "react";
import { IconButton, Popover, Typography } from "@mui/material";
import InfoOutlined from "@mui/icons-material/InfoOutlined";
import CloseRounded from "@mui/icons-material/CloseRounded";
const method = [
  [
    "定理の逆翻訳",
    "翻訳用の大規模言語モデル（LLM）にLeanの定理を渡し、前提と結論を自然文にします。元の依頼文を見せず、定理の記述に基づいて翻訳します。",
  ],
  [
    "依頼との意味比較",
    "比較用の別のLLMに元の依頼文と逆翻訳を渡し、条件の抜け、余分な前提、数値や対象の違いを評価させます。",
  ],
  [
    "差分と影響の提示",
    "差がある条件と、その条件で結果が変わる例を表示します。たとえば在庫が0個のときに購入できるかを示します。",
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
          claimcheckの二段階方式を参考にした実装案です。サンプル画面には、あらかじめ用意した比較例を表示します。
        </Typography>
        <ol>
          {method.map(([title, text]) => (
            <li key={title}>
              <strong>{title}</strong>
              <p>{text}</p>
            </li>
          ))}
        </ol>
        <a
          className="semantic-method-link"
          href="https://github.com/metareflection/claimcheck"
          target="_blank"
          rel="noreferrer"
        >
          claimcheckの二段階方式
        </a>
      </Popover>
    </>
  );
}
