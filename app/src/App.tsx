import { useEffect, useState } from "react";
import { api } from "./api";
import { RevContext, useRoute } from "./hooks";
import { HistoryView } from "./views/HistoryView";
import { QuestionView } from "./views/QuestionView";
import { TermsView } from "./views/TermsView";
import { TermView } from "./views/TermView";
import { ReviewView } from "./views/ReviewView";
import { SearchView } from "./views/SearchView";
import { SettingsView } from "./views/SettingsView";
import { TagsView } from "./views/TagsView";
import { StatsView } from "./views/StatsView";

const NAV = [
  { key: "history", label: "履歴", match: ["history", "q"] },
  { key: "review", label: "復習", match: ["review"] },
  { key: "terms", label: "用語", match: ["terms", "term", "tags"] },
  { key: "stats", label: "統計", match: ["stats"] },
  { key: "search", label: "検索", match: ["search"] },
  { key: "settings", label: "設定", match: ["settings"] },
];

/** MCP サーバーの書き込みを 1 秒ごとに確かめ、変わっていたら画面を読み直す（2 秒以内の反映）。 */
function useLiveRevision(): number {
  const [rev, setRev] = useState(0);
  useEffect(() => {
    let last: number | null = null;
    const tick = async () => {
      try {
        const v = await api.dataVersion();
        if (last !== null && v !== last) setRev((r) => r + 1);
        last = v;
      } catch {
        /* 起動直後などは無視 */
      }
    };
    void tick();
    const id = setInterval(tick, 1000);
    return () => clearInterval(id);
  }, []);
  return rev;
}

export function App() {
  const route = useRoute();
  const rev = useLiveRevision();
  const [head, arg] = route;
  const id = Number(arg);

  let view;
  switch (head) {
    case "q":
      view = <QuestionView id={id} />;
      break;
    case "terms":
      view = <TermsView key={arg ?? ""} initialTag={arg ?? ""} />;
      break;
    case "tags":
      view = <TagsView />;
      break;
    case "stats":
      view = <StatsView />;
      break;
    case "term":
      view = <TermView id={id} />;
      break;
    case "review":
      view = <ReviewView />;
      break;
    case "search":
      view = <SearchView />;
      break;
    case "settings":
      view = <SettingsView />;
      break;
    default:
      view = <HistoryView />;
  }

  return (
    <RevContext.Provider value={rev}>
      <div className="shell">
        <nav className="side" aria-label="メイン">
          <div className="brand">
            <span className="brand-mark" aria-hidden>
              <i /><i /><i />
            </span>
            Mondeep
          </div>
          <ul>
            {NAV.map((n) => (
              <li key={n.key}>
                <a href={`#/${n.key}`} aria-current={n.match.includes(head) ? "page" : undefined}>{n.label}</a>
              </li>
            ))}
          </ul>
        </nav>
        <main className="main">{view}</main>
      </div>
    </RevContext.Provider>
  );
}
