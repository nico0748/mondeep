// 図の種類ごとの配置規則と描画。種類を足すときは renderXxx を書いて RENDERERS に登録する。
import {
  BG, FONT, INK, LABEL_SIZE, MUTED, PALETTE, RULE, SIDE_LABEL, SMALL_SIZE,
  edgeLabel, edgeLabelSize, el, legend, markers, multiline, sideOf, textWidth, wrap,
  type LegendItem, type SvgNode,
} from "./core";
import type { ComparisonDiagram, ConceptDiagram, Diagram, SequenceDiagram, StateDiagram, Transition } from "./types";

// マス目（Rust 側の CELL_W / CELL_H と同じ）
export const CELL_W = 220;
export const CELL_H = 130;
const BOX_W = 168;
const BOX_H = 70;
const PAD = 24;
const TITLE_H = 40;
const LABEL_ROOM = 18;

export interface Rendered {
  svg: SvgNode;
  width: number;
  height: number;
}

let uid = 0;

function frame(d: { title?: string }, contentW: number, contentH: number, body: SvgNode[], legendItems: LegendItem[], prefix: string): Rendered {
  const titleH = d.title ? TITLE_H : 0;
  const lg = legend(legendItems, PAD, PAD + titleH + contentH + 16);
  const width = Math.max(contentW + PAD * 2, lg.width + PAD * 2, d.title ? textWidth(d.title, 18) + PAD * 2 : 0);
  const height = PAD + titleH + contentH + (lg.height ? lg.height + 28 : 8) + PAD / 2;
  const children: SvgNode[] = [markers(prefix), el("rect", { x: 0, y: 0, width, height, fill: BG })];
  if (d.title) {
    children.push(el("text", { x: PAD, y: PAD + 14, "font-size": 18, "font-weight": 700, "font-family": FONT.display, fill: INK }, [d.title]));
  }
  children.push(el("g", { transform: `translate(${PAD},${PAD + titleH})` }, body));
  children.push(lg.node);
  return {
    width,
    height,
    svg: el("svg", { xmlns: "http://www.w3.org/2000/svg", viewBox: `0 0 ${width} ${height}`, width, height, "font-family": FONT.body, role: "img" }, children),
  };
}

function sideLegend(sides: Set<string>, highlight: boolean, lines: { solid?: boolean; dashed?: boolean; dashedLabel?: string } = {}): LegendItem[] {
  const items: LegendItem[] = [];
  for (const s of ["self", "external", "error"]) {
    if (sides.has(s)) items.push({ kind: "box", palette: sideOf(s), label: SIDE_LABEL[s] });
  }
  if (highlight) items.push({ kind: "box", palette: PALETTE.highlight, label: "核心" });
  if (lines.dashed) items.push({ kind: "dash", palette: { fill: INK, stroke: INK, text: INK }, label: lines.dashedLabel ?? "補助の流れ" });
  return items;
}

type Pt = { x: number; y: number };

/** 箱の中心 c から目標 t へ向かう線が、箱の縁と交わる点。 */
function clipToBox(c: Pt, t: Pt, w: number, h: number): Pt {
  const dx = t.x - c.x;
  const dy = t.y - c.y;
  if (dx === 0 && dy === 0) return c;
  const s = Math.min(dx === 0 ? Infinity : w / 2 / Math.abs(dx), dy === 0 ? Infinity : h / 2 / Math.abs(dy));
  return { x: c.x + dx * s, y: c.y + dy * s };
}

/** マス目（または上書き座標）から中心を求める。Rust 側の diagram::center と同じ計算。 */
export function gridCenter(n: { row: number; col: number; x?: number; y?: number }): Pt {
  return { x: n.x ?? n.col * CELL_W + CELL_W / 2, y: n.y ?? n.row * CELL_H + CELL_H / 2 };
}

/**
 * 矢印ラベルの置き場所。箱と箱のすき間に収まるなら線の中央、収まらなければ箱の外
 * （横向きの線は箱の上、逆向きの 2 本目は下。縦向きの線は右）に置いて、箱に重ならないようにする。
 */
function placeLabel(n: number, label: string, a: Pt, b: Pt, p1: Pt, p2: Pt, boxW: number, boxH: number, second: boolean, highlight: boolean): SvgNode {
  const size = edgeLabelSize(label);
  const mx = (p1.x + p2.x) / 2;
  const my = (p1.y + p2.y) / 2;
  const dx = Math.abs(b.x - a.x);
  const dy = Math.abs(b.y - a.y);
  if (dx >= dy) {
    if (dx - boxW - 12 >= size.w || dy > boxH) return edgeLabel(n, label, mx, my + (second ? 4 : -4), highlight, { anchor: second ? "top" : "bottom" });
    return second
      ? edgeLabel(n, label, mx, Math.max(a.y, b.y) + boxH / 2 + 6, highlight, { anchor: "top" })
      : edgeLabel(n, label, mx, Math.min(a.y, b.y) - boxH / 2 - 6, highlight, { anchor: "bottom" });
  }
  if (dy - boxH - 12 >= size.h) return edgeLabel(n, label, mx + (second ? -6 : 6), my, highlight, { anchor: second ? "center" : "left" });
  return edgeLabel(n, label, Math.max(a.x, b.x) + boxW / 2 + 6, my, highlight, { anchor: "left" });
}

/** a→b に垂直な向きへ off だけずらす量。逆向きの線は符号が反転するので自然に離れる。 */
function parallelOffset(a: Pt, b: Pt, off: number): { nx: number; ny: number } {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const len = Math.hypot(dx, dy) || 1;
  return { nx: (-dy / len) * off, ny: (dx / len) * off };
}

function bounds(points: Pt[], padX: number, padY: number) {
  const xs = points.map((p) => p.x);
  const ys = points.map((p) => p.y);
  return { w: Math.max(...xs) + padX, h: Math.max(...ys) + padY };
}

// ------------------------------------------------------------- 概念図

export function renderConcept(d: ConceptDiagram): Rendered {
  const prefix = `zk${++uid}`;
  // 箱の上に置くラベルのぶん、全体を少し下げる（平行移動なので交差の判定は Rust 側と変わらない）
  const pos = new Map(d.nodes.map((n) => {
    const c = gridCenter(n);
    return [n.id, { x: c.x, y: c.y + LABEL_ROOM }];
  }));
  const body: SvgNode[] = [];

  for (const g of d.groups ?? []) {
    const pts = g.members.map((m) => pos.get(m)).filter((p): p is Pt => !!p);
    if (!pts.length) continue;
    const x1 = Math.min(...pts.map((p) => p.x)) - BOX_W / 2 - 16;
    const y1 = Math.min(...pts.map((p) => p.y)) - BOX_H / 2 - 28;
    const x2 = Math.max(...pts.map((p) => p.x)) + BOX_W / 2 + 16;
    const y2 = Math.max(...pts.map((p) => p.y)) + BOX_H / 2 + 14;
    const pal = sideOf(g.side);
    body.push(el("g", { class: "zk-group" }, [
      el("rect", { x: x1, y: y1, width: x2 - x1, height: y2 - y1, rx: 14, fill: "none", stroke: pal.stroke, "stroke-width": 1.2, "stroke-dasharray": "6 4", opacity: 0.8 }),
      el("text", { x: x1 + 12, y: y1 + 14, "font-size": SMALL_SIZE, "font-weight": 700, fill: pal.stroke, "dominant-baseline": "central" }, [g.label]),
    ]));
  }

  const edges = d.edges ?? [];
  const labels: SvgNode[] = [];
  edges.forEach((e, i) => {
    const a = pos.get(e.from);
    const b = pos.get(e.to);
    if (!a || !b) return;
    // 逆向きの矢印と重ならないよう、互いに少し横へずらす
    const { nx, ny } = parallelOffset(a, b, edges.some((o) => o.from === e.to && o.to === e.from) ? 10 : 0);
    const p1 = clipToBox({ x: a.x + nx, y: a.y + ny }, { x: b.x + nx, y: b.y + ny }, BOX_W + 8, BOX_H + 8);
    const p2 = clipToBox({ x: b.x + nx, y: b.y + ny }, { x: a.x + nx, y: a.y + ny }, BOX_W + 8, BOX_H + 8);
    const color = e.highlight ? PALETTE.highlight.stroke : INK;
    body.push(el("line", {
      x1: p1.x, y1: p1.y, x2: p2.x, y2: p2.y, stroke: color, "stroke-width": e.highlight ? 2.4 : 1.6,
      "stroke-dasharray": e.dashed ? "6 4" : undefined, "marker-end": `url(#${prefix}-${e.highlight ? "arrow-hl" : "arrow"})`,
    }));
    const second = edges.findIndex((o) => o.from === e.to && o.to === e.from) > -1 && edges.findIndex((o) => o.from === e.to && o.to === e.from) < i;
    labels.push(placeLabel(i + 1, e.label, a, b, p1, p2, BOX_W, BOX_H, second, !!e.highlight));
  });

  for (const n of d.nodes) {
    const c = pos.get(n.id)!;
    const pal = sideOf(n.side, n.highlight);
    const nameLines = wrap(n.label, BOX_W - 16, LABEL_SIZE, n.role ? 1 : 2);
    const kids: SvgNode[] = [
      el("rect", { x: c.x - BOX_W / 2, y: c.y - BOX_H / 2, width: BOX_W, height: BOX_H, rx: 10, fill: pal.fill, stroke: pal.stroke, "stroke-width": n.highlight ? 2.4 : 1.6 }),
    ];
    if (n.role) {
      kids.push(multiline(nameLines, c.x, c.y - 11, LABEL_SIZE, { "font-weight": 700, fill: pal.text }));
      kids.push(multiline(wrap(n.role, BOX_W - 14, SMALL_SIZE, 1), c.x, c.y + 13, SMALL_SIZE, { fill: MUTED }));
    } else {
      kids.push(multiline(nameLines, c.x, c.y, LABEL_SIZE, { "font-weight": 700, fill: pal.text }));
    }
    body.push(el("g", { class: "zk-node", "data-id": n.id }, kids));
  }
  body.push(...labels);

  const b = bounds([...pos.values()], BOX_W / 2 + 60, BOX_H / 2 + 20);
  const sides = new Set(d.nodes.filter((n) => !n.highlight).map((n) => n.side ?? "self"));
  const hl = d.nodes.some((n) => n.highlight) || edges.some((e) => e.highlight);
  return frame(d, b.w, b.h, body, sideLegend(sides, hl, { dashed: edges.some((e) => e.dashed) }), prefix);
}

// ------------------------------------------------------------- シーケンス図

const LANE_W = 220;
const HEAD_H = 46;
const ROW_H = 50;
const FRAG_GAP = 26;

export function renderSequence(d: SequenceDiagram): Rendered {
  const prefix = `zk${++uid}`;
  const lane = new Map(d.participants.map((p, i) => [p.id, i * LANE_W + LANE_W / 2]));
  const f = d.fragment && d.fragment.start <= d.fragment.end && d.fragment.end < d.messages.length ? d.fragment : undefined;
  // 枠の上には見出しのぶん、枠の下には余白のぶん行をずらす
  const rowY = (i: number) => {
    let y = HEAD_H + 30 + i * ROW_H + ROW_H / 2;
    if (f && i >= f.start) y += FRAG_GAP;
    if (f && i > f.end) y += 12;
    if (f && f.else_start !== undefined && i >= f.else_start) y += 18;
    return y;
  };
  const bodyH = rowY(d.messages.length - 1) + ROW_H / 2 + 14;
  const width = d.participants.length * LANE_W;
  const body: SvgNode[] = [];

  if (f) {
    const y1 = rowY(f.start) - ROW_H / 2 - FRAG_GAP + 4;
    const y2 = rowY(f.end) + 16;
    const tag = `${f.kind} [${f.label}]`;
    const tw = textWidth(tag, SMALL_SIZE) + 18;
    body.push(el("g", { class: "zk-fragment" }, [
      el("rect", { x: 6, y: y1, width: width - 12, height: y2 - y1, rx: 6, fill: "none", stroke: RULE, "stroke-width": 1.2 }),
      el("path", { d: `M6,${y1} h${tw} v12 l-7,7 H6 z`, fill: "#F1F4F8", stroke: RULE }),
      el("text", { x: 14, y: y1 + 10, "font-size": SMALL_SIZE, "font-weight": 700, fill: MUTED, "dominant-baseline": "central" }, [tag]),
    ]));
    if (f.kind === "alt" && f.else_start !== undefined && f.else_start > f.start && f.else_start <= f.end) {
      const ey = rowY(f.else_start) - ROW_H / 2 - 10;
      body.push(el("line", { x1: 6, y1: ey, x2: width - 6, y2: ey, stroke: RULE, "stroke-dasharray": "5 4" }));
      body.push(el("text", { x: 14, y: ey + 10, "font-size": SMALL_SIZE, fill: MUTED, "dominant-baseline": "central" }, [`[${f.else_label ?? "else"}]`]));
    }
  }

  for (const p of d.participants) {
    const x = lane.get(p.id)!;
    const pal = sideOf(p.side);
    body.push(el("line", { x1: x, y1: HEAD_H, x2: x, y2: bodyH, stroke: RULE, "stroke-width": 1.2, "stroke-dasharray": "4 4" }));
    body.push(el("g", { class: "zk-node", "data-id": p.id }, [
      el("rect", { x: x - 80, y: 0, width: 160, height: HEAD_H, rx: 8, fill: pal.fill, stroke: pal.stroke, "stroke-width": 1.6 }),
      multiline(wrap(p.label, 146, LABEL_SIZE, 2), x, HEAD_H / 2, LABEL_SIZE, { "font-weight": 700, fill: pal.text }),
    ]));
  }

  d.messages.forEach((m, i) => {
    const x1 = lane.get(m.from);
    const x2 = lane.get(m.to);
    if (x1 === undefined || x2 === undefined) return;
    const y = rowY(i);
    const color = m.highlight ? PALETTE.highlight.stroke : INK;
    const style = m.style ?? "sync";
    const marker = `url(#${prefix}-${style === "sync" ? "arrow" : "open"}${m.highlight ? "-hl" : ""})`;
    const common = { stroke: color, "stroke-width": m.highlight ? 2.2 : 1.5, fill: "none", "stroke-dasharray": style === "reply" ? "6 4" : undefined, "marker-end": marker };
    const lineY = y + 10;
    if (x1 === x2) {
      body.push(el("path", { d: `M${x1},${lineY - 10} h36 v16 h-34`, ...common }));
      body.push(edgeLabel(i + 1, m.label, x1 + 40 + Math.min(LANE_W / 2, textWidth(m.label, SMALL_SIZE) / 2 + 16), lineY - 2, !!m.highlight, { maxWidth: LANE_W - 40 }));
    } else {
      const dir = x2 > x1 ? 1 : -1;
      body.push(el("line", { x1: x1 + dir * 2, y1: lineY, x2: x2 - dir * 3, y2: lineY, ...common }));
      body.push(edgeLabel(i + 1, m.label, (x1 + x2) / 2, lineY - 4, !!m.highlight, { maxWidth: Math.abs(x2 - x1) + 10, anchor: "bottom" }));
    }
  });

  const sides = new Set(d.participants.map((p) => p.side ?? "self"));
  const items = sideLegend(sides, d.messages.some((m) => m.highlight), { dashed: d.messages.some((m) => m.style === "reply"), dashedLabel: "応答" });
  return frame(d, width, bodyH, body, items, prefix);
}

// ------------------------------------------------------------- 状態遷移図

const STATE_W = 150;
const STATE_H = 54;

export function transitionLabel(t: Transition): string {
  return `${t.event}${t.guard ? ` [${t.guard}]` : ""}${t.action ? ` / ${t.action}` : ""}`;
}

export function renderState(d: StateDiagram): Rendered {
  const prefix = `zk${++uid}`;
  // 開始の黒丸を置くため、全体を右に 40px ずらす
  const OFFSET = 40;
  const pos = new Map(d.states.map((s) => {
    const c = gridCenter(s);
    return [s.id, { x: c.x + OFFSET, y: c.y + LABEL_ROOM }];
  }));
  const body: SvgNode[] = [];
  const labels: SvgNode[] = [];

  d.transitions.forEach((t, i) => {
    const a = pos.get(t.from);
    const b = pos.get(t.to);
    if (!a || !b) return;
    const color = t.highlight ? PALETTE.highlight.stroke : INK;
    const marker = `url(#${prefix}-${t.highlight ? "arrow-hl" : "arrow"})`;
    if (t.from === t.to) {
      const top = a.y - STATE_H / 2;
      body.push(el("path", { d: `M${a.x - 24},${top} C${a.x - 30},${top - 46} ${a.x + 30},${top - 46} ${a.x + 24},${top}`, fill: "none", stroke: color, "stroke-width": 1.5, "marker-end": marker }));
      labels.push(edgeLabel(i + 1, transitionLabel(t), a.x, top - 40, !!t.highlight));
      return;
    }
    // 逆向きの遷移と重ならないよう、少し横にずらす
    const hasReverse = d.transitions.some((o) => o.from === t.to && o.to === t.from);
    const { nx, ny } = parallelOffset(a, b, hasReverse ? 10 : 0);
    const p1 = clipToBox({ x: a.x + nx, y: a.y + ny }, { x: b.x + nx, y: b.y + ny }, STATE_W + 8, STATE_H + 8);
    const p2 = clipToBox({ x: b.x + nx, y: b.y + ny }, { x: a.x + nx, y: a.y + ny }, STATE_W + 8, STATE_H + 8);
    body.push(el("line", { x1: p1.x, y1: p1.y, x2: p2.x, y2: p2.y, stroke: color, "stroke-width": t.highlight ? 2.2 : 1.5, "marker-end": marker }));
    const rev = d.transitions.findIndex((o) => o.from === t.to && o.to === t.from);
    labels.push(placeLabel(i + 1, transitionLabel(t), a, b, p1, p2, STATE_W, STATE_H, rev > -1 && rev < i, !!t.highlight));
  });

  for (const s of d.states) {
    const c = pos.get(s.id)!;
    const pal = sideOf(s.side, s.highlight);
    const kids: SvgNode[] = [
      el("rect", { x: c.x - STATE_W / 2, y: c.y - STATE_H / 2, width: STATE_W, height: STATE_H, rx: STATE_H / 2, fill: pal.fill, stroke: pal.stroke, "stroke-width": s.highlight ? 2.4 : 1.6 }),
    ];
    if (s.final) {
      kids.push(el("rect", { x: c.x - STATE_W / 2 + 4, y: c.y - STATE_H / 2 + 4, width: STATE_W - 8, height: STATE_H - 8, rx: STATE_H / 2 - 4, fill: "none", stroke: pal.stroke, "stroke-width": 1.2 }));
    }
    kids.push(multiline(wrap(s.label, STATE_W - 20, LABEL_SIZE, 2), c.x, c.y, LABEL_SIZE, { "font-weight": 700, fill: pal.text }));
    if (s.initial) {
      const sx = c.x - STATE_W / 2 - 30;
      kids.push(el("circle", { cx: sx, cy: c.y, r: 7, fill: INK }));
      kids.push(el("line", { x1: sx + 7, y1: c.y, x2: c.x - STATE_W / 2 - 3, y2: c.y, stroke: INK, "stroke-width": 1.5, "marker-end": `url(#${prefix}-arrow)` }));
    }
    body.push(el("g", { class: "zk-node", "data-id": s.id }, kids));
  }
  body.push(...labels);

  const b = bounds([...pos.values()], STATE_W / 2 + 70, STATE_H / 2 + 20);
  const sides = new Set(d.states.filter((s) => !s.highlight).map((s) => s.side ?? "self"));
  const hl = d.states.some((s) => s.highlight) || d.transitions.some((t) => t.highlight);
  return frame(d, b.w, b.h, body, sideLegend(sides, hl), prefix);
}

// ------------------------------------------------------------- 比較表

export function renderComparison(d: ComparisonDiagram): Rendered {
  const prefix = `zk${++uid}`;
  const FIRST_W = 130;
  const COL_W = 210;
  const HEAD = 44;
  const hl = new Set((d.highlights ?? []).map((h) => `${h.row}:${h.col}`));
  const rowLines = d.rows.map((r) => r.cells.map((c) => wrap(c, COL_W - 20, 13, 3)));
  const rowH = rowLines.map((cells) => Math.max(44, ...cells.map((l) => l.length * 13 * 1.35 + 18)));
  const width = FIRST_W + d.columns.length * COL_W;
  const height = HEAD + rowH.reduce((a, b) => a + b, 0);
  const body: SvgNode[] = [el("rect", { x: 0, y: 0, width, height, rx: 8, fill: BG, stroke: RULE })];

  d.columns.forEach((c, i) => {
    const x = FIRST_W + i * COL_W;
    body.push(el("rect", { x, y: 0, width: COL_W, height: HEAD, fill: PALETTE.self.fill, stroke: RULE }));
    body.push(multiline(wrap(c, COL_W - 16, LABEL_SIZE, 2), x + COL_W / 2, HEAD / 2, LABEL_SIZE, { "font-weight": 700, fill: PALETTE.self.text }));
  });
  let y = HEAD;
  d.rows.forEach((r, ri) => {
    const h = rowH[ri];
    body.push(el("rect", { x: 0, y, width: FIRST_W, height: h, fill: "#F3F5F8", stroke: RULE }));
    body.push(multiline(wrap(r.label, FIRST_W - 16, 13, 2), FIRST_W / 2, y + h / 2, 13, { "font-weight": 700, fill: INK }));
    r.cells.forEach((_, ci) => {
      const x = FIRST_W + ci * COL_W;
      const on = hl.has(`${ri}:${ci}`);
      body.push(el("rect", { x, y, width: COL_W, height: h, fill: on ? PALETTE.highlight.fill : BG, stroke: on ? PALETTE.highlight.stroke : RULE, "stroke-width": on ? 2 : 1 }));
      body.push(multiline(rowLines[ri][ci], x + COL_W / 2, y + h / 2, 13, { fill: on ? PALETTE.highlight.text : INK, "font-weight": on ? 700 : 400 }));
    });
    y += h;
  });
  const items: LegendItem[] = hl.size ? [{ kind: "box", palette: PALETTE.highlight, label: "核心の違い" }] : [];
  return frame(d, width, height, body, items, prefix);
}

// ------------------------------------------------------------- 入口

const RENDERERS: { [K in Diagram["kind"]]: (d: Extract<Diagram, { kind: K }>) => Rendered } = {
  concept: renderConcept,
  sequence: renderSequence,
  state: renderState,
  comparison: renderComparison,
};

export const KIND_LABEL: Record<string, string> = { concept: "概念図", sequence: "シーケンス図", state: "状態遷移図", comparison: "比較表" };

/** 図解データを描く。未対応の種類・版は例外にする（呼び出し側でメッセージを出す）。 */
export function renderDiagram(data: unknown): Rendered {
  const d = data as Diagram;
  if (!d || typeof d !== "object") throw new Error("図解データが空です");
  if ((d as { version?: number }).version !== 1) throw new Error(`未対応の版です（version=${(d as { version?: number }).version}）`);
  const r = RENDERERS[d.kind] as ((x: Diagram) => Rendered) | undefined;
  if (!r) throw new Error(`未対応の図の種類です（${String((d as { kind?: string }).kind)}）`);
  return r(d);
}
