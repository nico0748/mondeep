import { useState } from "react";
import { api } from "../api";
import { go, useLoad } from "../hooks";
import { pct } from "../format";
import { ErrorNote } from "../components/bits";
import { DailyChart } from "../charts/DailyChart";
import { levelLabel } from "../charts/stats";
import type { Stats } from "../types";

const PERIODS = [
  { days: 7, label: "7 日" },
  { days: 30, label: "30 日" },
  { days: 90, label: "90 日" },
  { days: 365, label: "1 年" },
];

/** 段数の縦棒の最大の高さ（px）。 */
const LEVEL_BAR_MAX = 110;
const fmt1 = (x: number | null) => (x === null ? "—" : x.toFixed(1));

/** 分野（タグ）別の正答率。名前・棒・数字を 1 行に並べ、苦手な分野から見せる。 */
function TagMeters({ stats }: { stats: Stats }) {
  const rows = stats.by_tag
    .filter((t) => t.accuracy !== null)
    .sort((a, b) => (a.accuracy ?? 0) - (b.accuracy ?? 0) || b.counts.correct - a.counts.correct);
  if (!rows.length) return <p className="empty">この期間の回答はありません。</p>;
  return (
    <ul className="meters">
      {rows.map((t) => {
        const n = t.counts.correct + t.counts.partial + t.counts.incorrect;
        const detail = `正解 ${t.counts.correct}・部分正解 ${t.counts.partial}・不正解 ${t.counts.incorrect}`;
        return (
          <li key={t.tag} title={detail}>
            <button className="link meter-name" onClick={() => t.tag !== "（タグなし）" && go(`terms/${encodeURIComponent(t.tag)}`)}>{t.tag}</button>
            <span className="meter" role="img" aria-label={`${t.tag}: 正答率 ${pct(t.accuracy)}（${detail}）`}>
              <span className="meter-fill" style={{ width: `${Math.round((t.accuracy ?? 0) * 100)}%` }} />
            </span>
            <span className="meter-value">{pct(t.accuracy)}<span className="sub">{n} 回答</span></span>
          </li>
        );
      })}
    </ul>
  );
}

/** 深掘りで正解できた段数ごとの木の数（縦棒、値は棒の上）。 */
function LevelColumns({ hist }: { hist: number[] }) {
  const max = Math.max(1, ...hist);
  return (
    <div className="levels" role="list" aria-label="正解できた段数ごとの深掘りの数">
      {hist.map((n, i) => {
        const label = levelLabel(i, i === hist.length - 1);
        return (
          <div className="level" role="listitem" key={i} aria-label={`${label}まで正解: ${n} 本`}>
            <span className="level-value">{n}</span>
            <span className="level-bar" style={{ height: Math.round((n / max) * LEVEL_BAR_MAX) }} />
            <span className="level-label">{label}</span>
          </div>
        );
      })}
    </div>
  );
}

/** 学習統計（F-11）: 日別の問題数、分野別正答率、深掘りの平均到達段数。 */
export function StatsView() {
  const [days, setDays] = useState(30);
  const { data, error } = useLoad(() => api.stats(days), [days]);
  // 読み直し中は前の表示を薄くして残す（レイアウトを跳ねさせない）
  const stale = data !== null && data.days !== days;

  return (
    <section className="view stats-view">
      <header className="view-head">
        <h1>統計</h1>
        <div className="segmented" role="group" aria-label="期間">
          {PERIODS.map((p) => (
            <button key={p.days} className="seg" aria-pressed={days === p.days} onClick={() => setDays(p.days)}>{p.label}</button>
          ))}
        </div>
      </header>
      <ErrorNote error={error} />
      {!data && !error && <div aria-busy="true" />}
      {data && (
        <div className={stale ? "stale" : ""}>
          <p className="meta">{data.from.replaceAll("-", "/")} 〜 {data.to.replaceAll("-", "/")}</p>
          <div className="tiles">
            <div className="tile"><span className="tile-label">回答</span><span className="tile-value">{data.counts.correct + data.counts.partial + data.counts.incorrect}</span><span className="tile-sub">{data.term_count} 語・{data.question_count} 問</span></div>
            <div className="tile"><span className="tile-label">正答率</span><span className="tile-value">{pct(data.accuracy)}</span><span className="tile-sub">部分正解は半分で計算</span></div>
            <div className="tile"><span className="tile-label">学習した日</span><span className="tile-value">{data.active_days}<small> / {data.days} 日</small></span></div>
            <div className="tile"><span className="tile-label">連続</span><span className="tile-value">{data.streak_days}<small> 日</small></span><span className="tile-sub">今日か昨日まで続いている日数</span></div>
          </div>

          <div className="block">
            <h2>日別の回答数</h2>
            {data.counts.correct + data.counts.partial + data.counts.incorrect === 0 ? (
              <p className="empty">この期間の回答はありません。Claude Desktop で出題してもらうと、ここに積み上がります。</p>
            ) : (
              <DailyChart daily={data.daily} days={data.days} />
            )}
          </div>

          <div className="block">
            <h2>分野別の正答率</h2>
            <p className="hint">正答率の低い分野から並べています。分野名を押すと、その分野の用語を一覧します。</p>
            <TagMeters stats={data} />
          </div>

          <div className="block">
            <h2>深掘り</h2>
            {data.deep_trees === 0 ? (
              <p className="empty">この期間に「用語→意味」の出題はありません。</p>
            ) : (
              <>
                <div className="tiles">
                  <div className="tile"><span className="tile-label">深掘りの回数</span><span className="tile-value">{data.deep_trees}<small> 回</small></span><span className="tile-sub">「用語→意味」で最初の問いから始めた数</span></div>
                  <div className="tile"><span className="tile-label">平均の到達段数</span><span className="tile-value">{fmt1(data.avg_reached_levels)}<small> 段</small></span><span className="tile-sub">最初の問いを 1 段と数える</span></div>
                  <div className="tile"><span className="tile-label">平均の正解段数</span><span className="tile-value">{fmt1(data.avg_correct_levels)}<small> 段</small></span><span className="tile-sub">正解できた最も深い段まで</span></div>
                </div>
                <h3 className="sub-head">どこまで正解できたか</h3>
                <LevelColumns hist={data.correct_level_hist} />
              </>
            )}
          </div>
        </div>
      )}
    </section>
  );
}
