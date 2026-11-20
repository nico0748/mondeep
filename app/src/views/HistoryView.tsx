import { useState } from "react";
import { api } from "../api";
import { go, useLoad } from "../hooks";
import { fmtDate, pct } from "../format";
import { FORMAT_LABEL, VERDICT_LABEL, type Format, type HistoryFilter, type Verdict } from "../types";
import { ErrorNote, Tags, VerdictMark, orderTree } from "../components/bits";

export function HistoryView() {
  const [filter, setFilter] = useState<HistoryFilter>({ limit: 50 });
  const { data, error, reload } = useLoad(() => api.history(filter), [JSON.stringify(filter)]);
  const tags = useLoad(() => api.allTags(), []);

  const set = (patch: Partial<HistoryFilter>) => setFilter((f) => ({ ...f, ...patch }));
  const remove = async (id: number, theme: string) => {
    if (!window.confirm(`「${theme || "無題のセッション"}」と、その問題・回答・解説をすべて削除します。元に戻せません。`)) return;
    await api.deleteSession(id);
    reload();
  };

  return (
    <section className="view">
      <header className="view-head">
        <h1>履歴</h1>
        <div className="filters" role="group" aria-label="絞り込み">
          <select value={filter.format ?? ""} onChange={(e) => set({ format: (e.target.value || undefined) as Format | undefined })} aria-label="形式">
            <option value="">すべての形式</option>
            <option value="meaning_to_term">{FORMAT_LABEL.meaning_to_term}</option>
            <option value="term_to_meaning">{FORMAT_LABEL.term_to_meaning}</option>
          </select>
          <select value={filter.tag ?? ""} onChange={(e) => set({ tag: e.target.value || undefined })} aria-label="タグ">
            <option value="">すべてのタグ</option>
            {(tags.data ?? []).map((t) => (
              <option key={t} value={t}>{t}</option>
            ))}
          </select>
          <select value={filter.verdict ?? ""} onChange={(e) => set({ verdict: (e.target.value || undefined) as Verdict | undefined })} aria-label="正誤">
            <option value="">すべての正誤</option>
            {(Object.keys(VERDICT_LABEL) as Verdict[]).map((v) => (
              <option key={v} value={v}>{VERDICT_LABEL[v]}</option>
            ))}
          </select>
        </div>
      </header>
      <ErrorNote error={error} />
      {data && data.length === 0 && (
        <div className="empty">
          <p>まだ記録がありません。</p>
          <p>Claude Desktop で「TLS 周りを意味→用語で 10 問」のように頼むと、問題と解説がここに残ります。設定画面から MCP の登録を済ませてください。</p>
        </div>
      )}
      <div className="sessions">
        {(data ?? []).map(({ session: s, questions }) => (
          <article className="session" key={s.id}>
            <header>
              <div>
                <h2>{s.theme || "無題のセッション"}</h2>
                <p className="meta">
                  {fmtDate(s.started_at)}　{FORMAT_LABEL[s.format]}　{s.question_count} 問　正答率 {pct(s.accuracy ?? (s.question_count ? (s.correct + s.partial / 2) / Math.max(1, s.correct + s.partial + s.incorrect) : null))}
                  {!s.ended_at && <span className="live">進行中</span>}
                </p>
                <Tags tags={s.tags} />
              </div>
              <button className="link danger" onClick={() => remove(s.id, s.theme)}>削除</button>
            </header>
            {s.summary && <p className="summary">{s.summary}</p>}
            <ul className="question-list">
              {orderTree(questions).map((q) => (
                <li key={q.id} style={{ ["--depth" as string]: q.depth }}>
                  <button className="question-row" onClick={() => go(`q/${q.id}`)}>
                    <span className="q-term">{q.headword}</span>
                    <span className="q-prompt">{q.depth > 0 && <span className="q-depth">{q.depth} 段目</span>}{q.prompt}</span>
                    <VerdictMark v={q.latest_verdict} />
                  </button>
                </li>
              ))}
            </ul>
          </article>
        ))}
      </div>
      {data && data.length >= (filter.limit ?? 50) && (
        <button className="more" onClick={() => set({ limit: (filter.limit ?? 50) + 50 })}>さらに読み込む</button>
      )}
    </section>
  );
}
