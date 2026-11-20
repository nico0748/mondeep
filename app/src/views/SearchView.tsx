import { useEffect, useState } from "react";
import { api } from "../api";
import { go, useLoad } from "../hooks";
import { ErrorNote } from "../components/bits";

const KIND_LABEL = { term: "用語", question: "問題文", note: "メモ", explanation: "解説" } as const;

export function SearchView() {
  const [input, setInput] = useState("");
  const [keyword, setKeyword] = useState("");
  useEffect(() => {
    const t = setTimeout(() => setKeyword(input), 200);
    return () => clearTimeout(t);
  }, [input]);
  const { data, error } = useLoad(() => (keyword.trim() ? api.search(keyword) : Promise.resolve([])), [keyword]);
  return (
    <section className="view">
      <header className="view-head">
        <h1>検索</h1>
      </header>
      <input className="search-box" type="search" autoFocus placeholder="用語・問題文・解説・メモを検索" value={input} onChange={(e) => setInput(e.target.value)} aria-label="検索キーワード" />
      <ErrorNote error={error} />
      {keyword.trim() && data && data.length === 0 && <p className="empty">「{keyword}」を含む記録はありません。</p>}
      <ul className="hits">
        {(data ?? []).map((h, i) => (
          <li key={i}>
            <button className="hit" onClick={() => go(h.question_id ? `q/${h.question_id}` : `term/${h.term_id}`)}>
              <span className="hit-kind">{KIND_LABEL[h.kind]}</span>
              <span className="t-head">{h.headword}</span>
              <span className="hit-snippet">{highlight(h.snippet, keyword)}</span>
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}

function highlight(text: string, kw: string) {
  const k = kw.trim();
  if (!k) return text;
  const idx = text.toLowerCase().indexOf(k.toLowerCase());
  if (idx < 0) return text;
  return (
    <>
      {text.slice(0, idx)}
      <mark>{text.slice(idx, idx + k.length)}</mark>
      {text.slice(idx + k.length)}
    </>
  );
}
