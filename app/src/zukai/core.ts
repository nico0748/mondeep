// 図解フレームワークの土台: SVG の要素木、文字幅の見積もり、配色・フォントの規約、凡例。
// AI が書いた SVG・HTML は使わず、すべてここで組み立てる。文字列は必ずエスケープされる。

export interface SvgNode {
  tag: string;
  attrs: Record<string, string | number>;
  children: (SvgNode | string)[];
}

export function el(tag: string, attrs: Record<string, string | number | undefined | false> = {}, children: (SvgNode | string)[] = []): SvgNode {
  const clean: Record<string, string | number> = {};
  for (const [k, v] of Object.entries(attrs)) {
    if (v !== undefined && v !== false) clean[k] = v;
  }
  return { tag, attrs: clean, children };
}

const ESC: Record<string, string> = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" };
export function escapeXml(s: string): string {
  return s.replace(/[&<>"']/g, (c) => ESC[c]);
}

const ALLOWED_TAGS = new Set(["svg", "g", "rect", "line", "path", "circle", "text", "tspan", "defs", "marker", "polygon", "title"]);

/** 要素木を SVG 文字列にする（書き出し用）。許可した要素だけを出す。 */
export function toSvgString(node: SvgNode): string {
  if (!ALLOWED_TAGS.has(node.tag)) throw new Error(`unexpected tag ${node.tag}`);
  const attrs = Object.entries(node.attrs)
    .filter(([k]) => /^[a-zA-Z:-]+$/.test(k) && !k.toLowerCase().startsWith("on"))
    .map(([k, v]) => ` ${k}="${escapeXml(String(v))}"`)
    .join("");
  const inner = node.children.map((c) => (typeof c === "string" ? escapeXml(c) : toSvgString(c))).join("");
  return `<${node.tag}${attrs}>${inner}</${node.tag}>`;
}

// ---- 規約: 色（青＝主役、オレンジ＝外部、赤＝エラー系、ピンク＝核心の強調）

export interface SidePalette {
  fill: string;
  stroke: string;
  text: string;
}
export const PALETTE: Record<"self" | "external" | "error" | "highlight", SidePalette> = {
  self: { fill: "#EAF1FB", stroke: "#2563A8", text: "#173E6B" },
  external: { fill: "#FFF2E5", stroke: "#DB7A1C", text: "#7A3F05" },
  error: { fill: "#FDECEC", stroke: "#C62828", text: "#7F1414" },
  highlight: { fill: "#FFE3EE", stroke: "#D6336C", text: "#8A1240" },
};
export const INK = "#26303D";
export const MUTED = "#5E6B7A";
export const RULE = "#AEB8C4";
export const BG = "#FFFFFF";

// ---- 規約: フォントは 3 役（魅せる・読ませる・コード）
export const FONT = {
  display: "'Hiragino Mincho ProN', 'Yu Mincho', 'Noto Serif JP', serif",
  body: "'Hiragino Sans', 'Hiragino Kaku Gothic ProN', 'Yu Gothic UI', 'Meiryo', 'Noto Sans JP', sans-serif",
  code: "'SF Mono', 'Menlo', 'Consolas', 'DejaVu Sans Mono', monospace",
};

export const LABEL_SIZE = 14;
export const SMALL_SIZE = 12;

/** 文字幅の見積もり（全角 1em、半角 0.6em）。 */
export function textWidth(s: string, size: number): number {
  let w = 0;
  for (const ch of s) w += /[\u0000-ÿ｡-ﾟ]/.test(ch) ? 0.6 : 1;
  return w * size;
}

/** 幅に収まるよう文字単位で折り返す（最大 maxLines 行、超えたら … で切る）。 */
export function wrap(s: string, maxWidth: number, size: number, maxLines = 2): string[] {
  const lines: string[] = [];
  let cur = "";
  for (const ch of s) {
    if (textWidth(cur + ch, size) > maxWidth && cur) {
      lines.push(cur);
      cur = ch.trimStart();
      if (lines.length === maxLines) break;
    } else {
      cur += ch;
    }
  }
  if (lines.length < maxLines) {
    if (cur) lines.push(cur);
  } else {
    lines[maxLines - 1] = lines[maxLines - 1].replace(/.$/u, "…");
  }
  return lines;
}

/** 複数行テキスト（中央揃え）。 */
export function multiline(lines: string[], x: number, y: number, size: number, attrs: Record<string, string | number> = {}): SvgNode {
  const lh = size * 1.3;
  const top = y - ((lines.length - 1) * lh) / 2;
  return el(
    "text",
    { x, y: top, "font-size": size, "text-anchor": "middle", "dominant-baseline": "central", ...attrs },
    lines.map((l, i) => el("tspan", { x, dy: i === 0 ? 0 : lh }, [l])),
  );
}

export function sideOf(side?: string, highlight?: boolean): SidePalette {
  if (highlight) return PALETTE.highlight;
  if (side === "external") return PALETTE.external;
  if (side === "error") return PALETTE.error;
  return PALETTE.self;
}

/** 矢印の先端の定義（通常・強調・開き矢印）。 */
export function markers(prefix: string): SvgNode {
  const head = (id: string, color: string, open = false) =>
    el("marker", { id: `${prefix}-${id}`, viewBox: "0 0 10 10", refX: 9, refY: 5, markerWidth: 8, markerHeight: 8, orient: "auto-start-reverse" }, [
      open
        ? el("path", { d: "M1,1 L9,5 L1,9", fill: "none", stroke: color, "stroke-width": 1.6 })
        : el("path", { d: "M0,0 L10,5 L0,10 z", fill: color }),
    ]);
  return el("defs", {}, [head("arrow", INK), head("arrow-hl", PALETTE.highlight.stroke), head("open", INK, true), head("open-hl", PALETTE.highlight.stroke, true)]);
}

/** ラベルの大きさの見積もり（配置を決めるため）。 */
export function edgeLabelSize(label: string, maxWidth = 150): { w: number; h: number } {
  const lines = wrap(label, maxWidth, SMALL_SIZE, 2);
  return { w: Math.max(...lines.map((l) => textWidth(l, SMALL_SIZE))) + 26, h: lines.length * SMALL_SIZE * 1.3 + 6 };
}

/** 矢印の番号バッジとラベル（白地の上に描いて線と重ならないようにする）。 */
export function edgeLabel(
  n: number | null, label: string, x: number, y: number, highlight: boolean,
  opts: { maxWidth?: number; anchor?: "center" | "bottom" | "top" | "left" } = {},
): SvgNode {
  const lines = wrap(label, opts.maxWidth ?? 150, SMALL_SIZE, 2);
  const w = Math.max(...lines.map((l) => textWidth(l, SMALL_SIZE))) + (n !== null ? 26 : 10);
  const h = lines.length * SMALL_SIZE * 1.3 + 6;
  // bottom のときは y を下端として、線に重ならない位置に置く
  if (opts.anchor === "bottom") y -= h / 2;
  if (opts.anchor === "top") y += h / 2;
  if (opts.anchor === "left") x += w / 2;
  const color = highlight ? PALETTE.highlight.stroke : INK;
  const children: SvgNode[] = [el("rect", { x: x - w / 2, y: y - h / 2, width: w, height: h, rx: 4, fill: BG, opacity: 0.94 })];
  let tx = x;
  if (n !== null) {
    const cx = x - w / 2 + 10;
    children.push(el("circle", { cx, cy: y, r: 8, fill: color }));
    children.push(el("text", { x: cx, y, "font-size": 10, "font-weight": 700, fill: "#fff", "text-anchor": "middle", "dominant-baseline": "central" }, [String(n)]));
    tx = x + 8;
  }
  children.push(multiline(lines, tx, y, SMALL_SIZE, { fill: color }));
  return el("g", { class: "zk-edge-label" }, children);
}

export interface LegendItem {
  kind: "box" | "line" | "dash";
  palette: SidePalette;
  label: string;
}

/** 凡例（使われている色・線だけを自動で並べる）。高さを返す。 */
export function legend(items: LegendItem[], x: number, y: number): { node: SvgNode; height: number; width: number } {
  if (items.length === 0) return { node: el("g"), height: 0, width: 0 };
  const children: SvgNode[] = [];
  let cx = x;
  for (const it of items) {
    if (it.kind === "box") {
      children.push(el("rect", { x: cx, y: y + 3, width: 16, height: 12, rx: 3, fill: it.palette.fill, stroke: it.palette.stroke, "stroke-width": 1.5 }));
    } else {
      children.push(el("line", { x1: cx, y1: y + 9, x2: cx + 16, y2: y + 9, stroke: it.palette.stroke, "stroke-width": 1.8, "stroke-dasharray": it.kind === "dash" ? "4 3" : undefined }));
    }
    children.push(el("text", { x: cx + 22, y: y + 9, "font-size": 11, fill: MUTED, "dominant-baseline": "central" }, [it.label]));
    cx += 22 + textWidth(it.label, 11) + 18;
  }
  return { node: el("g", { class: "zk-legend" }, children), height: 18, width: cx - x };
}

export const SIDE_LABEL: Record<string, string> = { self: "主役・自分側", external: "外部", error: "エラー・攻撃" };
