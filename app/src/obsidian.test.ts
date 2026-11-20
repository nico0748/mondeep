import { describe, expect, it } from "vitest";
import { buildExport, demoteHeadings, safeFileName, ASSET_DIR, INDEX_NOTE } from "./obsidian";
import type { ExportFile, TermExport } from "./types";

const text = (f: ExportFile) => new TextDecoder().decode(Uint8Array.from(atob(f.data_base64), (c) => c.charCodeAt(0)));

const concept = {
  version: 1,
  kind: "concept",
  title: "XSS の流れ",
  nodes: [
    { id: "attacker", label: "攻撃者", side: "external", row: 0, col: 0 },
    { id: "site", label: "脆弱なサイト", row: 0, col: 1, highlight: true },
  ],
  edges: [{ from: "attacker", to: "site", label: "スクリプト" }],
};

function sample(headword = "クロスサイトスクリプティング", tags = ["Web", "面接 対策"]): TermExport {
  return {
    term: { id: 1, headword, aliases: ["XSS"], tags, needs_review: true, created_at: "2026-11-20T00:00:00Z" },
    counts: { correct: 1, partial: 0, incorrect: 0 },
    accuracy: 1,
    review_states: [{ term_id: 1, format: "term_to_meaning", due_date: "2026-11-21", interval_days: 1, ease: 2.5, streak: 1, last_reviewed_at: null }],
    adjustments: [{ term_id: 1, kind: "concept", element_id: "site", dx: 40, dy: 0, updated_at: "" }],
    questions: [
      {
        id: 12, session_id: 3, term_id: 1, format: "term_to_meaning", prompt: "XSS とは？\n一言で", model_answer: null,
        parent_id: null, depth: 0, created_at: "2026-11-20T01:00:00Z", note: "SameSite との違いを\n復習", session_theme: "Web の基礎",
        attempts: [{ id: 5, question_id: 12, answer: "スクリプトを埋め込む攻撃", verdict: "correct", reason: "要点を押さえている", answered_at: "2026-11-20T01:01:00Z", original_verdict: "partial", overridden_at: "2026-11-20T02:00:00Z" }],
        explanation: {
          id: 7, question_id: 12, attempt_id: 5, created_at: "",
          body_md: "## 仕組み\n本文\n```\n# コード内の見出しは下げない\n```",
          diagrams: [concept],
          sources: [{ kind: "obsidian", name: "Web/XSS [メモ]" }, { kind: "web", name: "OWASP", url: "https://owasp.org/www-community/attacks/xss/" }],
        },
      },
    ],
  };
}

describe("Obsidian への書き出し", () => {
  it("用語 1 つをノート 1 枚と図の SVG にする", () => {
    const files = buildExport([sample()], new Date("2026-11-20T03:00:00Z"));
    expect(files.map((f) => f.path)).toEqual(["クロスサイトスクリプティング.md", `${ASSET_DIR}/クロスサイトスクリプティング-12-1.svg`]);
    const md = text(files[0]);
    expect(md).toMatch(/^---\naliases: \["XSS"\]\ntags: \["Web","面接_対策"\]\nmondeep_term_id: 1\n/);
    expect(md).toContain("# クロスサイトスクリプティング");
    expect(md).toContain("- 次回復習（用語→意味＋深掘り）: 2026-11-21");
    expect(md).toContain("> [!question] 問題\n> XSS とは？\n> 一言で");
    expect(md).toContain("正解（Claude の判定「部分正解」から手で修正）");
    expect(md).toContain("> [!note] メモ\n> SameSite との違いを\n> 復習");
    // 解説の見出しは「#### 解説」の下に入るよう下げる。コードブロックの中はそのまま
    expect(md).toContain("###### 仕組み");
    expect(md).toContain("# コード内の見出しは下げない");
    expect(md).toContain("![[クロスサイトスクリプティング-12-1.svg]]");
    expect(md).toContain("*図: XSS の流れ*");
    expect(md).toContain("- [[Web/XSS  メモ]]（Obsidian）");
    expect(md).toContain("- [OWASP](https://owasp.org/www-community/attacks/xss/)（Web）");
    // 図は微調整を反映して SVG になる
    const svg = text(files[1]);
    expect(svg.startsWith("<svg")).toBe(true);
    expect(svg).toContain("脆弱なサイト");
  });

  it("全用語なら目次を付け、同じファイル名にならないようにする", () => {
    const files = buildExport([sample("A/B"), sample("A:B", []), sample(INDEX_NOTE, [])]);
    const notes = files.filter((f) => f.path.endsWith(".md")).map((f) => f.path);
    expect(notes).toEqual(["A_B.md", "A_B (2).md", `${INDEX_NOTE} (2).md`, `${INDEX_NOTE}.md`]);
    const index = text(files.find((f) => f.path === `${INDEX_NOTE}.md`)!);
    expect(index).toContain("## Web\n\n- [[A_B]]");
    expect(index).toContain("## （タグなし）\n\n- [[A_B (2)]]");
  });

  it("描けない図は飛ばして、理由を書く", () => {
    const ex = sample();
    ex.questions[0].explanation!.diagrams = [{ version: 1, kind: "unknown" }];
    const files = buildExport([ex]);
    expect(files).toHaveLength(1);
    expect(text(files[0])).toContain("（図 1 は描けませんでした");
  });

  it("ファイル名とアプリ側の検査がずれない", () => {
    expect(safeFileName("../etc/passwd")).toBe("_etc_passwd");
    expect(safeFileName("TLS 1.3（ハンドシェイク）")).toBe("TLS 1.3_ハンドシェイク_");
    expect(safeFileName("...")).toBe("無題");
    expect(demoteHeadings("# a\n####### b\n~~~\n# c\n~~~", 4)).toBe("##### a\n####### b\n~~~\n# c\n~~~");
  });
});
