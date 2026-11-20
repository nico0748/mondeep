import { useState } from "react";
import { api } from "../api";
import { go, useLoad } from "../hooks";
import { fmtDay, pct } from "../format";
import { Counts, ErrorNote, Tags } from "../components/bits";

export function TermsView() {
  const [query, setQuery] = useState("");
  const [tag, setTag] = useState("");
  const { data, error } = useLoad(() => api.listTerms(tag, query), [tag, query]);
  const tags = useLoad(() => api.allTags(), []);
  return (
    <section className="view">
      <header className="view-head">
        <h1>用語</h1>
        <div className="filters">
          <input type="search" placeholder="用語・別表記で絞り込む" value={query} onChange={(e) => setQuery(e.target.value)} aria-label="用語で絞り込む" />
          <select value={tag} onChange={(e) => setTag(e.target.value)} aria-label="タグ">
            <option value="">すべてのタグ</option>
            {(tags.data ?? []).map((t) => <option key={t} value={t}>{t}</option>)}
          </select>
        </div>
      </header>
      <ErrorNote error={error} />
      {data && data.length === 0 && <p className="empty">該当する用語はありません。</p>}
      <table className="term-table">
        <thead>
          <tr><th>用語</th><th>正誤</th><th>正答率</th><th>次回復習</th></tr>
        </thead>
        <tbody>
          {(data ?? []).map((t) => (
            <tr key={t.id} onClick={() => go(`term/${t.id}`)} tabIndex={0} onKeyDown={(e) => e.key === "Enter" && go(`term/${t.id}`)}>
              <td>
                <span className="t-head">{t.headword}</span>
                {t.needs_review && <span className="flag">要復習</span>}
                {t.aliases.length > 0 && <span className="t-alias">{t.aliases.join("、")}</span>}
                <Tags tags={t.tags} />
              </td>
              <td><Counts c={t.counts} /></td>
              <td className="num">{pct(t.accuracy)}</td>
              <td>{fmtDay(t.next_due)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
}
