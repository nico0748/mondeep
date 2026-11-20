import { useState } from "react";
import { api } from "../api";
import { go, isComposing, useLoad } from "../hooks";
import { fmtDate } from "../format";
import { FORMAT_LABEL, SOURCE_LABEL, VERDICT_LABEL, type Attempt, type Verdict } from "../types";
import { DeepLadder, ErrorNote, Tags, VerdictMark } from "../components/bits";
import { Markdown } from "../components/Markdown";
import { DiagramFigure } from "../components/DiagramFigure";

const VERDICTS: Verdict[] = ["correct", "partial", "incorrect"];

/** 判定の手動修正（F-14）。元の判定は残し、いつでも Claude の判定に戻せる。 */
function VerdictFix({ attempt: a, onDone }: { attempt: Attempt; onDone: () => void }) {
  const [open, setOpen] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const set = async (v: Verdict | null) => {
    try {
      await api.overrideVerdict(a.id, v);
      setErr(null);
      setOpen(false);
      onDone();
    } catch (e) {
      setErr(`直せませんでした: ${e}`);
    }
  };
  return (
    <div className="verdict-fix">
      {open ? (
        <div className="segmented" role="group" aria-label="判定を直す">
          {VERDICTS.map((v) => (
            <button key={v} className={`seg ${v}`} aria-pressed={a.verdict === v} onClick={() => set(v)}>{VERDICT_LABEL[v]}</button>
          ))}
          {a.original_verdict && <button className="link" onClick={() => set(null)}>Claude の判定に戻す</button>}
          <button className="link" onClick={() => setOpen(false)}>やめる</button>
        </div>
      ) : (
        <button className="link" onClick={() => setOpen(true)}>判定を直す</button>
      )}
      {err && <p className="error">{err}</p>}
    </div>
  );
}

/** 問題へのメモ（F-14）。検索でき、Claude も get_term_history で読む。 */
function NoteEditor({ questionId, note, onDone }: { questionId: number; note: string | null; onDone: () => void }) {
  const [draft, setDraft] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const save = async () => {
    if (saving || draft === null) return;
    // 保存中は入力を止める（保存の完了で、そのあとに打った文字を消さないように）。失敗したら下書きは残す
    setSaving(true);
    try {
      await api.setQuestionNote(questionId, draft);
      setDraft(null);
      setErr(null);
      onDone();
    } catch (e) {
      setErr(`保存できませんでした: ${e}`);
    } finally {
      setSaving(false);
    }
  };
  return (
    <div className="block note-block">
      <h2>メモ</h2>
      {draft !== null ? (
        <>
          <textarea
            className="note-input"
            value={draft}
            autoFocus
            rows={4}
            maxLength={10000}
            disabled={saving}
            placeholder="納得いかない点、覚え方、次に聞いてほしいことなど"
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={(e) => !isComposing(e) && (e.metaKey || e.ctrlKey) && e.key === "Enter" && save()}
            aria-label="メモ"
          />
          <div className="actions">
            <button onClick={save} disabled={saving}>{saving ? "保存中…" : "保存"}</button>
            <button className="link" onClick={() => setDraft(null)} disabled={saving}>やめる</button>
          </div>
        </>
      ) : note ? (
        <>
          <p className="note-text">{note}</p>
          <button className="link" onClick={() => setDraft(note)}>メモを直す</button>
        </>
      ) : (
        <button className="link" onClick={() => setDraft("")}>メモを書く</button>
      )}
      {err && <p className="error">{err}</p>}
      <p className="hint">メモは検索でき、Claude も次に出題するときに読みます。</p>
    </div>
  );
}

export function QuestionView({ id }: { id: number }) {
  const { data, error, reload } = useLoad(() => api.questionDetail(id), [id]);
  if (error) return <section className="view"><ErrorNote error={error} /></section>;
  if (!data) return <section className="view" aria-busy="true" />;
  const { question: q, term, session, attempts, explanation, tree } = data;
  const latest = attempts[attempts.length - 1];

  return (
    <section className="view question">
      <nav className="crumbs">
        <button className="link" onClick={() => go("history")}>履歴</button>
        <span>{session.theme || "無題のセッション"}</span>
      </nav>
      <header className="question-head">
        <button className="headword small" onClick={() => go(`term/${term.id}`)}>{term.headword}</button>
        <p className="meta">{FORMAT_LABEL[q.format]}　{q.depth === 0 ? "最初の問い" : `深掘り ${q.depth} 段目`}　{fmtDate(q.created_at)}</p>
        <Tags tags={term.tags} />
      </header>

      <div className="qa">
        <div className="qa-block">
          <h2>問題</h2>
          <p className="prompt">{q.prompt}</p>
          {q.model_answer && <p className="model-answer"><span>模範解答</span>{q.model_answer}</p>}
        </div>
        {attempts.length === 0 && <p className="empty">まだ回答が記録されていません。</p>}
        {attempts.map((a) => (
          <div className={`qa-block attempt ${a.verdict}`} key={a.id}>
            <h2>
              自分の回答 <VerdictMark v={a.verdict} />
              {a.original_verdict && <span className="overridden">Claude の判定「{VERDICT_LABEL[a.original_verdict]}」から修正</span>}
            </h2>
            <p className="answer">{a.answer}</p>
            {a.reason && <p className="reason"><span>判定の理由</span>{a.reason}</p>}
            <div className="attempt-foot">
              <p className="meta">{fmtDate(a.answered_at)}</p>
              <VerdictFix attempt={a} onDone={reload} />
            </div>
          </div>
        ))}
      </div>

      <NoteEditor key={q.id} questionId={q.id} note={q.note} onDone={reload} />

      {tree.length > 1 && (
        <div className="block">
          <h2>深掘り</h2>
          <DeepLadder tree={tree} current={q.id} />
        </div>
      )}

      {explanation && (
        <article className="block explanation">
          <h2>解説</h2>
          {explanation.body_md && <Markdown source={explanation.body_md} />}
          {explanation.diagrams.map((d, i) => (
            <DiagramFigure key={i} data={d} termId={term.id} name={`mondeep-${term.headword}-${q.id}-${i + 1}`} />
          ))}
          {explanation.sources.length > 0 && (
            <div className="sources">
              <h3>出典</h3>
              <ul>
                {explanation.sources.map((s, i) => (
                  <li key={i}>
                    <span className="source-kind">{SOURCE_LABEL[s.kind] ?? s.kind}</span>
                    {s.url ? <button className="link" onClick={() => api.openExternal(s.url!)}>{s.name}</button> : <span>{s.name}</span>}
                  </li>
                ))}
              </ul>
            </div>
          )}
        </article>
      )}
      {!explanation && latest && <p className="empty">この問題には解説がありません。</p>}
    </section>
  );
}
