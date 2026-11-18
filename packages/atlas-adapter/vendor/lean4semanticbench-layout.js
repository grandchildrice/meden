/*!
MIT License

Copyright (c) 2026 0xfab

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/
// Extracted from 0xfab0131/Lean4SemanticBench, Graph/Web/js/graph.js.
// Commit: 97e7c29df8d3c358c395bd1a4f2fb6973527b33d. MIT; see LICENSE.Lean4SemanticBench.
function longestPathLevels(root, adj, hop) {
  const inSet = new Set(Object.keys(hop));
  const indeg = {};
  for (const v of inSet) indeg[v] = 0;
  for (const u of inSet)
    for (const v of adj[u] || []) if (inSet.has(v)) indeg[v]++;
  const level = {};
  level[root] = 0;
  const queue = [];
  for (const v of inSet)
    if (indeg[v] === 0) {
      if (v !== root) level[v] = hop[v];
      queue.push(v);
    }
  if (!queue.includes(root)) queue.push(root);
  const settled = new Set();
  for (let qi = 0; qi < queue.length; qi++) {
    const u = queue[qi];
    if (settled.has(u)) continue;
    settled.add(u);
    for (const v of adj[u] || []) {
      if (!inSet.has(v)) continue;
      const cand = (level[u] !== undefined ? level[u] : hop[u]) + 1;
      if (level[v] === undefined || cand > level[v]) level[v] = cand;
      if (--indeg[v] <= 0) queue.push(v);
    }
  }
  // 巡回（整礎再帰の補助定義など）に巻き込まれた分は BFS ホップ数で退避
  for (const v of inSet) if (level[v] === undefined) level[v] = hop[v];
  const layers = [];
  for (const [v, l] of Object.entries(level))
    (layers[l] || (layers[l] = [])).push(v);
  for (let i = 0; i < layers.length; i++) if (!layers[i]) layers[i] = [];
  return { level, layers };
}

/* ---------- 円錐配置 ----------

     直交格子に層を積むと、扇入り／扇出しの大きい宣言のところで辺が束になって
     互いに重なり、どれがどこへ行くのか読めなくなる。中心から放射する構造は
     本来「中心の上のファイバー束」なので、その形のまま極座標に置く。

       ・層 ℓ は半径 r(ℓ) の円弧に載る（＝ファイバーの底空間が角度方向）
       ・角度は親の角度の重心で並べる（交差削減）
       ・半径は「その層の全ノードが弧上で重ならない最小値」まで自動で開く
         → 密な層ほど円錐が大きく開く。重なりが原理的に生じない。
       ・辺は極座標のベジエ（中間半径で角度だけ移る）で描くので、
         同じ扇に属する辺が束ねられ、交差が視覚的に整理される。

     戻り値の pos は画面座標 (x, y) と極座標 (r, th) の両方を持つ。
     辺の描画は極座標側を使う。 */
function layoutCone(spec) {
  const {
    layers,
    adj,
    th0,
    th1,
    groupOf,
    // 層上限より優先して必ず置く節点の集合（経路表示の区間・手動展開）。null なら無し
    pin = null,
    // dr の既定は √(slotMax² + ノード高²) を上回らせる(隣接リングの
    // 軸平行矩形が どの角度でも重ならないための必要条件。store.js 参照)
    r0 = 150,
    dr = 200,
    slotMax = 172,
    slotMin = 62,
    slotPref = 196,
    rSoft = 1150,
    maxPerLayer = 48,
  } = spec;
  const span = Math.max(0.15, th1 - th0);
  const mid = (th0 + th1) / 2;

  const levelOf = {};
  layers.forEach((L, l) =>
    L.forEach((n) => {
      levelOf[n] = l;
    }),
  );
  /* parents    外向き（層が増える）の親。重心法の角度決めに使う。
       anyParent  向きを問わない親。巡回の退避（longestPathLevels の hop 代用）では
                  親の層が自分以上になり得て、その辺は canvas が非外向きの直線で描く。 */
  const parents = {},
    anyParent = {};
  for (const u of Object.keys(levelOf))
    for (const v of adj[u] || []) {
      if (levelOf[v] === undefined || v === u) continue;
      (anyParent[v] || (anyParent[v] = [])).push(u);
      if (levelOf[v] > levelOf[u]) (parents[v] || (parents[v] = [])).push(u);
    }

  const pos = {};
  const rings = [];
  const overflow = {};
  const angle = {};
  let r = r0 - dr;
  let rMax = 0;
  /* ここまでに置いた節点。親が一つも置かれていない節点は辺を引けず浮くので
       置かず、溢れに数える。浮く原因は二つある。
         ・層上限で親が溢れた（深さ 2 以上の子が孤立する）
         ・親が全て別の錐に取られた／種別フィルタで落ちた（この錐の層に親が無い）
       外向きの親が無いのは巡回の退避の節点で、親がこの錐に残っていれば後の層で
       置かれ、辺は非外向きとして描かれるので許す。根は中心として必ず描かれる。 */
  const placed = new Set(layers[0] || []);

  for (let l = 1; l < layers.length; l++) {
    const full = layers[l] || [];
    if (!full.length) continue;
    const layer = full.filter((n) => {
      const ps = parents[n] || [];
      if (ps.length) return ps.some((p) => placed.has(p));
      return (anyParent[n] || []).length > 0;
    });
    if (!layer.length) {
      overflow[l] = full.length;
      continue;
    }

    // 重心法: 親の角度の平均。親は必ず前の層にあるので既に確定している。
    const key = {};
    for (const n of layer) {
      const ps = (parents[n] || [])
        .map((p) => angle[p])
        .filter((a) => a !== undefined);
      key[n] = ps.length ? ps.reduce((a, b) => a + b, 0) / ps.length : mid;
    }
    /* 同じ部分グラフ（名前空間・主題・種別）に属するものを弧の上で隣り合わせる。
         主鍵を「その群の重心の平均」にすることで、群をまとめても交差が増えない。
         群の中の順序は従来どおり重心。 */
    let ordered;
    if (groupOf) {
      const gKey = new Map(),
        gCnt = new Map();
      for (const n of layer) {
        const g = groupOf(n) || "";
        gKey.set(g, (gKey.get(g) || 0) + key[n]);
        gCnt.set(g, (gCnt.get(g) || 0) + 1);
      }
      for (const [g, s] of gKey) gKey.set(g, s / gCnt.get(g));
      ordered = layer.slice().sort((a, b) => {
        const ga = groupOf(a) || "",
          gb = groupOf(b) || "";
        if (ga !== gb) return gKey.get(ga) - gKey.get(gb) || (ga < gb ? -1 : 1);
        return key[a] - key[b] || (a < b ? -1 : 1);
      });
    } else {
      ordered = layer
        .slice()
        .sort((a, b) => key[a] - key[b] || (a < b ? -1 : 1));
    }

    /* 表示上限。溢れた分は右パネルの一覧で全件見られるので、ここでは件数だけ伝える。
         経路表示や手動展開で「見せる」と決めた節点（pin）は上限より優先し、
         角度の順序は保ったまま、残りの枠を先頭から埋める。 */
    let shown = ordered;
    if (ordered.length > maxPerLayer) {
      const must = new Set(pin ? ordered.filter((n) => pin.has(n)) : []);
      const keep = new Set(must);
      let room = Math.max(0, maxPerLayer - must.size);
      for (const n of ordered) {
        if (room <= 0) break;
        if (!must.has(n)) {
          keep.add(n);
          room--;
        }
      }
      shown = ordered.filter((n) => keep.has(n));
    }
    if (full.length > shown.length) overflow[l] = full.length - shown.length;

    /* 半径の決め方。
         rNeed  これ未満だと弧上でノードが重なる、絶対の下限。
         rWant  一枚一枚を最大幅で置ける理想値。層が薄いときはこちらが効いて、
                数個しかない層が中心の近くで潰れて点になるのを防ぐ。
         rSoft  理想値の頭打ち。層が厚いときに半径が青天井になるのを止める。
                このとき slot は自動的に細くなり、ノードは小さく描かれる。 */
    const rNeed = (shown.length * slotMin) / span;
    const rWant = (shown.length * slotPref) / span;
    r = Math.max(r + dr, r0, rNeed, Math.min(rWant, rSoft));
    rMax = Math.max(rMax, r);

    const slot = Math.min(slotMax, (span * r) / Math.max(shown.length, 1));
    const usedSpan = (shown.length * slot) / r;
    const start = mid - usedSpan / 2 + slot / r / 2;
    shown.forEach((n, i) => {
      const th = start + (i * slot) / r;
      angle[n] = th;
      placed.add(n);
      pos[n] = {
        r,
        th,
        level: l,
        slot,
        x: r * Math.cos(th),
        y: r * Math.sin(th),
        w: Math.max(46, Math.min(slotMax - 14, slot - 12)),
      };
    });
    /* 同じ群が連続している区間を記録する。描画側はこれを弧の帯として描く。
         位置が確定した後で切り出すので、帯とノードが必ず一致する。 */
    let runs = [];
    if (groupOf) {
      const half = slot / r / 2;
      for (const n of shown) {
        const g = groupOf(n) || "";
        const last = runs[runs.length - 1];
        if (last && last.group === g) {
          last.th1 = pos[n].th + half;
          last.n++;
        } else
          runs.push({
            group: g,
            th0: pos[n].th - half,
            th1: pos[n].th + half,
            n: 1,
          });
      }
      runs = runs.filter((x) => x.group);
    }
    rings.push({
      level: l,
      r,
      count: full.length,
      shown: shown.length,
      th0,
      th1,
      slot,
      runs,
    });
  }
  return { pos, rings, overflow, rMax, th0, th1, mid };
}

export { longestPathLevels, layoutCone };
