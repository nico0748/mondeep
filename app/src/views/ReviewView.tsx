import { api } from "../api";
import { go, useLoad } from "../hooks";
import { fmtDay, pct } from "../format";
import { FORMAT_LABEL } from "../types";
import { Counts, ErrorNote, Tags } from "../components/bits";

export function ReviewView() {
  const queue = useLoad(() => api.reviewQueue(), []);
  const weak = useLoad(() => api.weakTerms(), []);
  const toggle = async (termId: number, on: boolean) => {
    await api.setNeedsReview(termId, on);
    queue.reload();
    weak.reload();
  };
  return (
    <section className="view">
      <header className="view-head"><h1>復習</h1></header>
      <div className="block">
        <h2>今日の復習</h2>
        <p className="hint">Claude Desktop で「今日の復習をやりたい」と頼むと、この一覧から出題されます。</p>
        <ErrorNote error={queue.error} />
        {queue.data && queue.data.length === 0 && <p className="empty">今日復習する用語はありません。</p>}
        <ul className="cards">
          {(queue.data ?? []).map((r) => (
            <li key={`${r.term_id}-${r.format}`} className="card">
              <button className="card-main" onClick={() => go(`term/${r.term_id}`)}>
                <span className="t-head">{r.headword}</span>
                <span className="meta">{r.format ? FORMAT_LABEL[r.format] : "形式未定"}{r.due_date && `　${fmtDay(r.due_date)} から`}</span>
                <span className="card-summary">{r.summary}</span>
              </button>
              <Tags tags={r.tags} />
              {r.needs_review && <button className="link" onClick={() => toggle(r.term_id, false)}>印を外す</button>}
            </li>
          ))}
        </ul>
      </div>
      <div className="block">
        <h2>苦手な用語</h2>
        <ErrorNote error={weak.error} />
        {weak.data && weak.data.length === 0 && <p className="empty">苦手な用語はまだありません。</p>}
        <table className="term-table">
          <thead><tr><th>用語</th><th>理由</th><th>正答率</th><th>正誤</th><th /></tr></thead>
          <tbody>
            {(weak.data ?? []).map((w) => (
              <tr key={w.term_id}>
                <td><button className="link t-head" onClick={() => go(`term/${w.term_id}`)}>{w.headword}</button></td>
                <td><ul className="reasons">{w.reasons.map((r) => <li key={r}>{r}</li>)}</ul></td>
                <td className="num">{pct(w.accuracy)}</td>
                <td><Counts c={w.counts} /></td>
                <td>
                  <button className={`flag-toggle small ${w.needs_review ? "on" : ""}`} onClick={() => toggle(w.term_id, !w.needs_review)} aria-pressed={w.needs_review}>
                    {w.needs_review ? "要復習" : "要復習にする"}
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}
