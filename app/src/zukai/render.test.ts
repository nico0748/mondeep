import { describe, expect, it } from "vitest";
import { renderDiagram, toSvgString, gridCenter, PALETTE } from "./index";

const concept = {
  version: 1,
  kind: "concept",
  title: "WAF の位置",
  nodes: [
    { id: "user", label: "利用者", side: "external", row: 0, col: 0 },
    { id: "waf", label: "WAF", role: "HTTP を検査", row: 0, col: 1, highlight: true },
    { id: "app", label: "Web アプリ", row: 0, col: 2 },
  ],
  edges: [
    { from: "user", to: "waf", label: "HTTP リクエスト" },
    { from: "waf", to: "app", label: "正常な通信だけ", dashed: true },
  ],
  groups: [{ id: "own", label: "自社環境", members: ["waf", "app"] }],
};

describe("図解フレームワーク", () => {
  it("概念図: 規約の色・番号付きラベル・凡例を描く", () => {
    const r = renderDiagram(concept);
    const svg = toSvgString(r.svg);
    expect(svg.startsWith("<svg")).toBe(true);
    expect(svg).toContain(PALETTE.external.stroke); // 外部＝オレンジ
    expect(svg).toContain(PALETTE.highlight.stroke); // 核心＝ピンク
    expect(svg).toContain("HTTP リクエスト");
    expect(svg).toContain(">1</text>");
    expect(svg).toContain(">2</text>");
    expect(svg).toContain("外部"); // 凡例
    expect(svg).toContain("自社環境");
    expect(r.width).toBeGreaterThan(3 * 220);
  });

  it("マス目の中心は Rust 側と同じ計算", () => {
    expect(gridCenter({ row: 1, col: 2 })).toEqual({ x: 2 * 220 + 110, y: 130 + 65 });
    expect(gridCenter({ row: 1, col: 2, x: 10, y: 20 })).toEqual({ x: 10, y: 20 });
  });

  it("文字はエスケープされ、AI が書いたマークアップは要素にならない", () => {
    const evil = { ...concept, nodes: [{ id: "x", label: '<script>alert(1)</script>"', row: 0, col: 0 }], edges: [], groups: [] };
    const svg = toSvgString(renderDiagram(evil).svg);
    expect(svg).not.toContain("<script>");
    expect(svg).toContain("&lt;script&gt;");
    const doc = new DOMParser().parseFromString(svg, "image/svg+xml");
    expect(doc.querySelector("script")).toBeNull();
  });

  it("シーケンス図: 応答は破線、枠を描く", () => {
    const svg = toSvgString(
      renderDiagram({
        version: 1,
        kind: "sequence",
        participants: [{ id: "c", label: "クライアント" }, { id: "s", label: "サーバー", side: "external" }],
        messages: [
          { from: "c", to: "s", label: "ClientHello" },
          { from: "s", to: "c", label: "ServerHello", style: "reply" },
          { from: "c", to: "c", label: "鍵を導出" },
        ],
        fragment: { kind: "alt", label: "証明書が有効", start: 1, end: 2, else_start: 2, else_label: "無効" },
      }).svg,
    );
    expect(svg).toContain('stroke-dasharray="6 4"');
    expect(svg).toContain("alt [証明書が有効]");
    expect(svg).toContain("[無効]");
    expect(svg).toContain("応答");
  });

  it("状態遷移図: イベント [ガード] / アクション のラベルと開始状態", () => {
    const svg = toSvgString(
      renderDiagram({
        version: 1,
        kind: "state",
        states: [
          { id: "a", label: "CLOSED", row: 0, col: 0, initial: true },
          { id: "b", label: "ESTABLISHED", row: 0, col: 1, final: true },
        ],
        transitions: [
          { from: "a", to: "b", event: "connect", guard: "SYN+ACK", action: "ACK 送信" },
          { from: "b", to: "a", event: "close" },
          { from: "b", to: "b", event: "データ" },
        ],
      }).svg,
    );
    // 長いラベルは折り返されるので、要素の文字をつないで確かめる
    const doc = new DOMParser().parseFromString(svg, "image/svg+xml");
    const labels = [...doc.querySelectorAll(".zk-edge-label text:last-child")].map((t) => t.textContent);
    expect(labels).toContain("connect [SYN+ACK] / ACK 送信");
    expect(svg).toContain("<circle");
  });

  it("比較表: 強調セルはピンク", () => {
    const svg = toSvgString(
      renderDiagram({
        version: 1,
        kind: "comparison",
        columns: ["共通鍵暗号", "公開鍵暗号"],
        rows: [{ label: "鍵の数", cells: ["1 つを共有", "公開鍵と秘密鍵"] }],
        highlights: [{ row: 0, col: 1 }],
      }).svg,
    );
    expect(svg).toContain(PALETTE.highlight.fill);
    expect(svg).toContain("核心の違い");
  });

  it("未対応の版・種類は例外", () => {
    expect(() => renderDiagram({ ...concept, version: 2 })).toThrow(/未対応の版/);
    expect(() => renderDiagram({ version: 1, kind: "svg" })).toThrow(/未対応の図の種類/);
  });
});

describe("位置の微調整", () => {
  const nodeRect = (svg: string, id: string) => {
    const doc = new DOMParser().parseFromString(svg, "image/svg+xml");
    const rect = doc.querySelector(`g.zk-node[data-id="${id}"] rect`)!;
    return { x: Number(rect.getAttribute("x")), y: Number(rect.getAttribute("y")) };
  };

  it("部品だけを指定量ずらし、他の部品は動かさない", () => {
    const base = toSvgString(renderDiagram(concept).svg);
    const moved = toSvgString(renderDiagram(concept, { offsets: { waf: { dx: 30, dy: 40 } } }).svg);
    const a = nodeRect(base, "waf");
    const b = nodeRect(moved, "waf");
    expect(b.x - a.x).toBe(30);
    expect(b.y - a.y).toBe(40);
    expect(nodeRect(moved, "app")).toEqual(nodeRect(base, "app"));
  });

  it("図の外（左上）へははみ出さない", () => {
    const moved = toSvgString(renderDiagram(concept, { offsets: { user: { dx: -999, dy: -999 } } }).svg);
    const r = nodeRect(moved, "user");
    expect(r.x).toBeGreaterThanOrEqual(0);
    expect(r.y).toBeGreaterThanOrEqual(0);
  });

  it("動かせるのは概念図と状態遷移図だけ", () => {
    expect(renderDiagram(concept).adjustable).toBe(true);
    expect(renderDiagram({ version: 1, kind: "comparison", columns: ["a"], rows: [{ label: "x", cells: ["y"] }] }).adjustable).toBe(false);
  });
});

describe("矢印ラベルの配置", () => {
  it("逆向きの 2 本の矢印のラベルは、それぞれの線の側に分かれて重ならない", () => {
    const d = {
      version: 1, kind: "concept",
      nodes: [{ id: "a", label: "A", row: 0, col: 0 }, { id: "b", label: "B", row: 0, col: 2 }],
      edges: [{ from: "a", to: "b", label: "要求" }, { from: "b", to: "a", label: "応答" }],
    };
    const doc = new DOMParser().parseFromString(toSvgString(renderDiagram(d).svg), "image/svg+xml");
    const boxes = [...doc.querySelectorAll(".zk-edge-label rect")].map((r) => ({ y: Number(r.getAttribute("y")), h: Number(r.getAttribute("height")) }));
    expect(boxes).toHaveLength(2);
    const [p, q] = boxes.sort((m, n) => m.y - n.y);
    expect(p.y + p.h).toBeLessThanOrEqual(q.y);
  });
});
