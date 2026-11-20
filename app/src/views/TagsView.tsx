import { useState } from "react";
import { api } from "../api";
import { go, useLoad } from "../hooks";
import { ErrorNote } from "../components/bits";

/** タグ・分野の管理（F-10）。表記ゆれをまとめたり、使わなくなったタグを外したりする。 */
export function TagsView() {
  const { data, error, reload } = useLoad(() => api.tagStats(), []);
  const [editing, setEditing] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [note, setNote] = useState<string | null>(null);

  const startEdit = (tag: string) => {
    setEditing(tag);
    setDraft(tag);
    setNote(null);
  };
  const rename = async (tag: string) => {
    const next = draft.trim();
    if (!next || next === tag) return setEditing(null);
    const merging = (data ?? []).find((t) => t.tag === next);
    if (merging && !window.confirm(`「${tag}」を「${next}」にまとめます。両方のタグが付いた用語は「${next}」だけになります。`)) return;
    try {
      const n = await api.renameTag(tag, next);
      setNote(`${n} 語のタグを「${next}」に${merging ? "まとめ" : "変え"}ました。`);
      setEditing(null);
      reload();
    } catch (e) {
      setNote(`変えられませんでした: ${e}`);
    }
  };
  const remove = async (tag: string, count: number) => {
    if (!window.confirm(`「${tag}」を ${count} 語から外します。用語や学習記録は消えません。`)) return;
    try {
      await api.removeTag(tag);
      setNote(`「${tag}」を外しました。`);
      reload();
    } catch (e) {
      setNote(`外せませんでした: ${e}`);
    }
  };

  return (
    <section className="view tags-view">
      <nav className="crumbs"><button className="link" onClick={() => go("terms")}>用語</button></nav>
      <header className="view-head"><h1>タグ</h1></header>
      <p className="hint">タグは Claude が保存するときに付けます。ここで名前を変えると、そのタグが付いたすべての用語に反映されます。同じ名前に変えると 1 つにまとまります。</p>
      <ErrorNote error={error} />
      {note && <p className="note" role="status">{note}</p>}
      {data && data.length === 0 && <p className="empty">まだタグがありません。</p>}
      {data && data.length > 0 && (
        <table className="term-table tag-table">
          <thead>
            <tr><th>タグ</th><th className="num">用語数</th><th aria-label="操作" /></tr>
          </thead>
          <tbody>
            {data.map((t) => (
              <tr key={t.tag}>
                <td>
                  {editing === t.tag ? (
                    <input
                      className="tag-rename"
                      value={draft}
                      autoFocus
                      maxLength={40}
                      onChange={(e) => setDraft(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") void rename(t.tag);
                        if (e.key === "Escape") setEditing(null);
                      }}
                      aria-label={`「${t.tag}」の新しい名前`}
                    />
                  ) : (
                    <button className="link tag-name" onClick={() => go(`terms/${encodeURIComponent(t.tag)}`)}>{t.tag}</button>
                  )}
                </td>
                <td className="num">{t.term_count}</td>
                <td>
                  <div className="row-actions">
                  {editing === t.tag ? (
                    <>
                      <button className="link" onClick={() => rename(t.tag)}>決定</button>
                      <button className="link" onClick={() => setEditing(null)}>やめる</button>
                    </>
                  ) : (
                    <>
                      <button className="link" onClick={() => startEdit(t.tag)}>名前を変える</button>
                      <button className="link danger" onClick={() => remove(t.tag, t.term_count)}>外す</button>
                    </>
                  )}
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}
