import { api } from "../api";
import { go, useLoad } from "../hooks";
import { fmtDate } from "../format";
import { FORMAT_LABEL, SOURCE_LABEL } from "../types";
import { DeepLadder, ErrorNote, Tags, VerdictMark } from "../components/bits";
import { Markdown } from "../components/Markdown";
import { DiagramFigure } from "../components/DiagramFigure";

export function QuestionView({ id }: { id: number }) {
  const { data, error } = useLoad(() => api.questionDetail(id), [id]);
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
            <h2>自分の回答 <VerdictMark v={a.verdict} /></h2>
            <p className="answer">{a.answer}</p>
            {a.reason && <p className="reason"><span>判定の理由</span>{a.reason}</p>}
            <p className="meta">{fmtDate(a.answered_at)}</p>
          </div>
        ))}
      </div>

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
