// 解説を Obsidian 用の Markdown に書き出す（F-13）。用語 1 つにつきノート 1 枚、図は SVG ファイルにして埋め込む。
import { renderDiagram, toSvgString, type Offsets } from "./zukai";
import { base64FromString } from "./encode";
import type { ExportFile, ExportQuestion, Source, TermExport, Verdict } from "./types";
import { FORMAT_LABEL, SOURCE_LABEL, VERDICT_LABEL } from "./types";

/** ダウンロード/Mondeep/ の下の書き出し先フォルダ（同名のファイルは上書きする）。 */
export const OBSIDIAN_FOLDER = "Obsidian";
/** 図の SVG を置くサブフォルダ。 */
export const ASSET_DIR = "図解";
/** 全用語を書き出すときの目次ノート。 */
export const INDEX_NOTE = "Mondeep 用語一覧";

/**
 * ファイル名に使える形にする。文字・数字と `-_.() ` だけを残し、ほかは `_` にする。
 * アプリ側（Rust）の検査を素通りする文字だけにしておくので、ノート内のリンクと実際のファイル名がずれない。
 */
export function safeFileName(name: string, max = 80): string {
  const s = name
    .normalize("NFC")
    .replace(/[^\p{L}\p{N}\-_.() ]/gu, "_")
    .replace(/\.{2,}/g, ".")
    .trim()
    .replace(/^\.+/, "")
    .slice(0, max)
    .trim();
  return s || "無題";
}

/** 同じ名前（大文字小文字を区別しないファイルシステムを考えて）にならないよう番号を付ける。 */
function uniqueName(base: string, used: Set<string>): string {
  let name = base;
  for (let i = 2; used.has(name.toLowerCase()); i++) name = `${base} (${i})`;
  used.add(name.toLowerCase());
  return name;
}

/** Markdown の見出しを `by` 段下げる（コードブロックの中はそのまま、最大 6 段）。 */
export function demoteHeadings(md: string, by: number): string {
  let fence: string | null = null;
  return md
    .split("\n")
    .map((line) => {
      const f = line.match(/^\s{0,3}(`{3,}|~{3,})/);
      if (f) {
        if (fence === null) fence = f[1][0];
        else if (f[1][0] === fence) fence = null;
        return line;
      }
      if (fence !== null) return line;
      return line.replace(/^(\s{0,3})(#{1,6})(?=\s|$)/, (_, sp: string, h: string) => sp + "#".repeat(Math.min(6, h.length + by)));
    })
    .join("\n");
}

const quote = (text: string) => text.split("\n").map((l) => `> ${l}`.trimEnd()).join("\n");
/** wikilink に使えない文字を除く。 */
const wikiSafe = (s: string) => s.replace(/[[\]|#^]/g, " ").trim();
/** YAML のフロー形式の配列（JSON は YAML として正しい）。 */
const yamlList = (items: string[]) => JSON.stringify(items);
/** Obsidian のタグは空白を含められない。 */
const obsidianTag = (t: string) => t.trim().replace(/\s+/g, "_").replace(/[#,]/g, "");

function day(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso.slice(0, 10);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}

function sourceLine(s: Source): string {
  const kind = SOURCE_LABEL[s.kind] ?? s.kind;
  if (s.url && /^https?:\/\//i.test(s.url)) return `- [${s.name.replace(/[[\]]/g, "")}](${s.url.replace(/[()\s]/g, encodeURIComponent)})（${kind}）`;
  if (s.kind === "obsidian" && wikiSafe(s.name)) return `- [[${wikiSafe(s.name)}]]（${kind}）`;
  return `- ${s.name}（${kind}）`;
}

function verdictText(v: Verdict, original: Verdict | null): string {
  return original ? `${VERDICT_LABEL[v]}（Claude の判定「${VERDICT_LABEL[original]}」から手で修正）` : VERDICT_LABEL[v];
}

interface Built {
  /** ノートのファイル名（拡張子なし） */
  name: string;
  files: ExportFile[];
}

/** 用語 1 つ分のノートと図のファイルを作る。`used` は書き出し全体で使ったファイル名。 */
export function buildTermNote(ex: TermExport, used: Set<string>, exportedAt: Date = new Date()): Built {
  const { term } = ex;
  const name = uniqueName(safeFileName(term.headword), used);
  const files: ExportFile[] = [];
  const offsetsFor = (kind: string): Offsets => {
    const o: Offsets = {};
    for (const a of ex.adjustments) if (a.kind === kind) o[a.element_id] = { dx: a.dx, dy: a.dy };
    return o;
  };

  const out: string[] = [];
  out.push("---");
  out.push(`aliases: ${yamlList(term.aliases)}`);
  out.push(`tags: ${yamlList(term.tags.map(obsidianTag).filter(Boolean))}`);
  out.push(`mondeep_term_id: ${term.id}`);
  out.push(`mondeep_exported: ${day(exportedAt.toISOString())}`);
  out.push("---", "");
  out.push(`# ${term.headword}`, "");
  if (term.aliases.length) out.push(`別表記: ${term.aliases.join(" / ")}`, "");
  const c = ex.counts;
  const acc = ex.accuracy === null ? "—" : `${Math.round(ex.accuracy * 100)}%`;
  out.push(`- 正答率: ${acc}（正解 ${c.correct}・部分正解 ${c.partial}・不正解 ${c.incorrect}）`);
  for (const r of ex.review_states) out.push(`- 次回復習（${FORMAT_LABEL[r.format]}）: ${r.due_date}`);
  if (term.needs_review) out.push("- 要復習の印あり");
  out.push("");

  if (ex.questions.length === 0) {
    out.push("まだ出題されていません。", "");
  } else {
    out.push("## 出題の記録", "");
    for (const q of ex.questions) out.push(...questionSection(q, name, files, offsetsFor));
  }

  files.unshift({ path: `${name}.md`, data_base64: base64FromString(out.join("\n").replace(/\n{3,}/g, "\n\n").trimEnd() + "\n") });
  return { name, files };
}

function questionSection(
  q: ExportQuestion,
  noteName: string,
  files: ExportFile[],
  offsetsFor: (kind: string) => Offsets,
): string[] {
  const out: string[] = [];
  const step = q.depth === 0 ? "最初の問い" : `深掘り ${q.depth} 段目`;
  out.push(`### ${day(q.created_at)} ${FORMAT_LABEL[q.format]}・${step}`, "");
  if (q.session_theme) out.push(`セッション: ${q.session_theme}`, "");
  out.push("> [!question] 問題", quote(q.prompt), "");
  if (q.model_answer) out.push("> [!example]- 模範解答", quote(q.model_answer), "");
  if (q.attempts.length === 0) out.push("（回答の記録なし）", "");
  for (const a of q.attempts) {
    out.push(`> [!quote] 自分の回答 — ${verdictText(a.verdict, a.original_verdict)}`, quote(a.answer || "（空欄）"), "");
    if (a.reason) out.push(`判定の理由: ${a.reason}`, "");
  }
  if (q.note) out.push("> [!note] メモ", quote(q.note), "");

  const e = q.explanation;
  if (e && (e.body_md.trim() || e.diagrams.length)) {
    out.push("#### 解説", "");
    if (e.body_md.trim()) out.push(demoteHeadings(e.body_md.trim(), 4), "");
    e.diagrams.forEach((d, i) => {
      const kind = (d as { kind?: string })?.kind ?? "";
      const title = (d as { title?: string })?.title;
      try {
        const r = renderDiagram(d, { offsets: offsetsFor(kind) });
        const file = `${noteName}-${q.id}-${i + 1}.svg`;
        files.push({ path: `${ASSET_DIR}/${file}`, data_base64: base64FromString(toSvgString(r.svg)) });
        out.push(`![[${file}]]`);
        if (title) out.push(`*図: ${title}*`);
        out.push("");
      } catch (err) {
        out.push(`（図 ${i + 1} は描けませんでした: ${err instanceof Error ? err.message : String(err)}）`, "");
      }
    });
    if (e.sources.length) out.push("出典:", ...e.sources.map(sourceLine), "");
  }
  return out;
}

/** 全用語の目次ノート（タグごと）。 */
export function buildIndexNote(notes: { name: string; ex: TermExport }[], used: Set<string>): ExportFile {
  const name = uniqueName(INDEX_NOTE, used);
  const byTag = new Map<string, string[]>();
  for (const { name: n, ex } of notes) {
    const tags = ex.term.tags.length ? ex.term.tags : ["（タグなし）"];
    for (const t of tags) byTag.set(t, [...(byTag.get(t) ?? []), n]);
  }
  const out = [`# ${INDEX_NOTE}`, "", `${notes.length} 語（Mondeep から書き出し）`, ""];
  for (const [tag, names] of [...byTag].sort(([a], [b]) => a.localeCompare(b, "ja"))) {
    out.push(`## ${tag}`, "", ...names.map((n) => `- [[${n}]]`), "");
  }
  return { path: `${name}.md`, data_base64: base64FromString(out.join("\n").trimEnd() + "\n") };
}

/** 書き出すファイル一式（1 語なら目次なし）。 */
export function buildExport(list: TermExport[], exportedAt: Date = new Date()): ExportFile[] {
  const used = new Set<string>();
  // 目次の名前を先に確保しておく（同名の用語があっても目次が上書きされないように）
  if (list.length > 1) used.add(INDEX_NOTE.toLowerCase());
  const notes = list.map((ex) => ({ ex, built: buildTermNote(ex, used, exportedAt) }));
  const files = notes.flatMap((n) => n.built.files);
  if (list.length > 1) {
    used.delete(INDEX_NOTE.toLowerCase());
    files.push(buildIndexNote(notes.map((n) => ({ name: n.built.name, ex: n.ex })), used));
  }
  return files;
}
