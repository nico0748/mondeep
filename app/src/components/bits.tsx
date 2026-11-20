import type { QuestionSummary, Verdict, VerdictCounts } from "../types";
import { VERDICT_LABEL } from "../types";
import { go } from "../hooks";

export function VerdictMark({ v }: { v: Verdict | null }) {
  if (!v) return <span className="verdict none">未回答</span>;
  return <span className={`verdict ${v}`}>{VERDICT_LABEL[v]}</span>;
}

export function Counts({ c }: { c: VerdictCounts }) {
  return (
    <span className="counts" aria-label={`正解 ${c.correct}、部分正解 ${c.partial}、不正解 ${c.incorrect}`}>
      <span className="c correct">{c.correct}</span>
      <span className="c partial">{c.partial}</span>
      <span className="c incorrect">{c.incorrect}</span>
    </span>
  );
}

/** 深掘りのはしご。段ごとに字下げし、どの深さで詰まったかが一目で分かるようにする。 */
export function DeepLadder({ tree, current }: { tree: QuestionSummary[]; current?: number }) {
  const ordered = orderTree(tree);
  return (
    <ol className="ladder">
      {ordered.map((q) => (
        <li key={q.id} style={{ ["--depth" as string]: q.depth }} className={q.id === current ? "current" : ""}>
          <button className="ladder-step" onClick={() => go(`q/${q.id}`)}>
            <span className={`rung ${q.latest_verdict ?? "none"}`} aria-hidden />
            <span className="ladder-depth">{q.depth === 0 ? "最初の問い" : `${q.depth} 段目`}</span>
            <span className="ladder-prompt">{q.prompt}</span>
            <VerdictMark v={q.latest_verdict} />
          </button>
        </li>
      ))}
    </ol>
  );
}

/** 親→子の順（深さ優先）に並べる。 */
export function orderTree(tree: QuestionSummary[]): QuestionSummary[] {
  const ids = new Set(tree.map((q) => q.id));
  const children = new Map<number | null, QuestionSummary[]>();
  for (const q of tree) {
    const p = q.parent_id !== null && ids.has(q.parent_id) ? q.parent_id : null;
    children.set(p, [...(children.get(p) ?? []), q]);
  }
  const out: QuestionSummary[] = [];
  const walk = (p: number | null) => {
    for (const q of children.get(p) ?? []) {
      out.push(q);
      walk(q.id);
    }
  };
  walk(null);
  return out;
}

export function Tags({ tags }: { tags: string[] }) {
  if (!tags.length) return null;
  return (
    <span className="tags">
      {tags.map((t) => (
        <span className="tag" key={t}>{t}</span>
      ))}
    </span>
  );
}

export function ErrorNote({ error }: { error: string | null }) {
  return error ? <p className="error">読み込めませんでした: {error}</p> : null;
}
