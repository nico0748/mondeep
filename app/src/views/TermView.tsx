import { useState } from "react";
import { api } from "../api";
import { go, useLoad } from "../hooks";
import { fmtDate, fmtDay, pct } from "../format";
import { FORMAT_LABEL } from "../types";
import { Counts, ErrorNote, VerdictMark } from "../components/bits";
import { Markdown } from "../components/Markdown";
import { DiagramFigure } from "../components/DiagramFigure";
import { buildExport, OBSIDIAN_FOLDER } from "../obsidian";

export function TermView({ id }: { id: number }) {
  const { data, error, reload } = useLoad(() => api.termCard(id), [id]);
  const [tagDraft, setTagDraft] = useState("");
  const [exportNote, setExportNote] = useState<string | null>(null);
  if (error) return <section className="view"><ErrorNote error={error} /></section>;
  if (!data) return <section className="view" aria-busy="true" />;
  const { term, counts, accuracy, review_states, questions, latest_explanation, max_correct_depth } = data;

  const toggle = async () => {
    await api.setNeedsReview(term.id, !term.needs_review);
    reload();
  };
  const addTag = async () => {
    const t = tagDraft.trim();
    if (!t) return;
    await api.setTermTags(term.id, [...term.tags, t]);
    setTagDraft("");
    reload();
  };
  const exportNoteFile = async () => {
    try {
      const files = buildExport([await api.termExport(term.id)]);
      const dir = await api.exportFiles(OBSIDIAN_FOLDER, files);
      setExportNote(`${dir} に書き出しました（ノート 1 枚${files.length > 1 ? `と図 ${files.length - 1} 枚` : ""}）。Vault にコピーすると Obsidian で開けます。`);
    } catch (e) {
      setExportNote(`書き出せませんでした: ${e}`);
    }
  };
  const removeTag = async (t: string) => {
    await api.setTermTags(term.id, term.tags.filter((x) => x !== t));
    reload();
  };

  return (
    <section className="view term">
      <nav className="crumbs"><button className="link" onClick={() => go("terms")}>用語</button></nav>
      <header className="entry">
        <h1 className="headword">{term.headword}</h1>
        {term.aliases.length > 0 && <p className="aliases">{term.aliases.join("　/　")}</p>}
        <div className="entry-tags">
          {term.tags.map((t) => (
            <span className="tag" key={t}>{t}<button className="tag-x" aria-label={`${t} を外す`} onClick={() => removeTag(t)}>×</button></span>
          ))}
          <input className="tag-input" value={tagDraft} placeholder="タグを追加" onChange={(e) => setTagDraft(e.target.value)} onKeyDown={(e) => e.key === "Enter" && addTag()} aria-label="タグを追加" />
        </div>
        <div className="entry-actions">
          <button className={`flag-toggle ${term.needs_review ? "on" : ""}`} onClick={toggle} aria-pressed={term.needs_review}>
            {term.needs_review ? "要復習の印を外す" : "要復習の印を付ける"}
          </button>
          <button className="more" onClick={exportNoteFile}>Obsidian 用に書き出す</button>
        </div>
        {exportNote && <p className="note export-note" role="status">{exportNote}</p>}
      </header>

      <dl className="stats">
        <div><dt>正答率</dt><dd>{pct(accuracy)}</dd></div>
        <div><dt>正誤</dt><dd><Counts c={counts} /></dd></div>
        <div><dt>深掘りで正解できた最も深い段</dt><dd>{max_correct_depth === null ? "—" : max_correct_depth === 0 ? "最初の問いまで" : `${max_correct_depth} 段目`}</dd></div>
        {review_states.map((r) => (
          <div key={r.format}><dt>次回復習（{FORMAT_LABEL[r.format]}）</dt><dd>{fmtDay(r.due_date)}<span className="sub">間隔 {r.interval_days} 日・連続正解 {r.streak}</span></dd></div>
        ))}
      </dl>

      {latest_explanation && (
        <article className="block explanation">
          <h2>最新の解説</h2>
          {latest_explanation.body_md && <Markdown source={latest_explanation.body_md} />}
          {latest_explanation.diagrams.map((d, i) => (
            <DiagramFigure key={i} data={d} termId={term.id} name={`mondeep-${term.headword}-${i + 1}`} />
          ))}
        </article>
      )}

      <div className="block">
        <h2>過去の出題</h2>
        <ul className="question-list">
          {questions.map((q) => (
            <li key={q.id} style={{ ["--depth" as string]: 0 }}>
              <button className="question-row" onClick={() => go(`q/${q.id}`)}>
                <span className="q-term">{fmtDate(q.created_at)}</span>
                <span className="q-prompt">{q.depth > 0 && <span className="q-depth">{q.depth} 段目</span>}{q.prompt}</span>
                <VerdictMark v={q.latest_verdict} />
              </button>
            </li>
          ))}
        </ul>
      </div>
    </section>
  );
}
