import { useEffect, useRef, useState } from "react";
import type { DailyStat } from "../types";
import { VERDICT_LABEL } from "../types";
import { bucketize, bucketLabel, bucketSize, niceTicks, SERIES, stackSegments, total, type Bucket } from "./stats";

const H = 200; // 描画域の高さ
const AXIS_W = 32;
const X_AXIS_H = 24;
const TOP = 8;
const GAP = 2; // 積み上げの間の面の色のすき間
const MAX_BAR = 24;

/** 要素の幅を追いかける（文字の大きさを保ったまま横幅に合わせて描くため）。 */
function useWidth<T extends HTMLElement>(): [React.RefObject<T | null>, number] {
  const ref = useRef<T>(null);
  const [w, setW] = useState(0);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    setW(el.clientWidth);
    if (typeof ResizeObserver === "undefined") return;
    const ro = new ResizeObserver(([e]) => setW(e.contentRect.width));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  return [ref, w];
}

/** 上端だけ角を丸めた棒（土台側は四角）。 */
function topRounded(x: number, y: number, w: number, h: number, r: number): string {
  const rr = Math.min(r, w / 2, h);
  return `M${x},${y + h}V${y + rr}Q${x},${y} ${x + rr},${y}H${x + w - rr}Q${x + w},${y} ${x + w},${y + rr}V${y + h}Z`;
}

/** 日別（期間が長ければ週別）の回答数を、判定ごとに積み上げた縦棒で見せる。 */
export function DailyChart({ daily, days }: { daily: DailyStat[]; days: number }) {
  const [ref, width] = useWidth<HTMLDivElement>();
  const [hover, setHover] = useState<number | null>(null);
  const size = bucketSize(days);
  const buckets = bucketize(daily, size);
  const max = Math.max(0, ...buckets.map((b) => total(b.counts)));
  const ticks = niceTicks(max);
  const top = ticks[ticks.length - 1];
  const plotW = Math.max(0, width - AXIS_W);
  const slot = buckets.length ? plotW / buckets.length : 0;
  const barW = Math.max(2, Math.min(MAX_BAR, slot - GAP));
  const y = (v: number) => TOP + H - (v / top) * H;
  // 日付の目盛りは 6 本ほどに間引き、最後（今日）は必ず出す
  const every = Math.max(1, Math.ceil(buckets.length / 6));
  const showLabel = (i: number) => i === buckets.length - 1 || (buckets.length - 1 - i) % every === 0;
  const describe = (b: Bucket) =>
    `${bucketLabel(b)}: ${SERIES.map((s) => `${VERDICT_LABEL[s]} ${b.counts[s]}`).join("、")}（計 ${total(b.counts)}）`;
  const hb = hover !== null ? buckets[hover] : null;

  return (
    <figure className="viz">
      <div className="viz-legend" aria-hidden>
        {SERIES.map((s) => (
          <span key={s}><i className={`swatch ${s}`} />{VERDICT_LABEL[s]}</span>
        ))}
        {size > 1 && <span className="viz-sub">1 本 = 1 週間</span>}
      </div>
      <div className="viz-plot" ref={ref} onPointerLeave={() => setHover(null)}>
        {width > 0 && (
          <svg width={width} height={TOP + H + X_AXIS_H} role="img" aria-label={`期間中の回答数（${size > 1 ? "週" : "日"}ごと、判定別）`}>
            {ticks.map((t) => (
              <g key={t} className="viz-grid">
                <line x1={AXIS_W} x2={width} y1={y(t)} y2={y(t)} />
                <text x={AXIS_W - 6} y={y(t)} dy="0.32em" textAnchor="end">{t}</text>
              </g>
            ))}
            {buckets.map((b, i) => {
              const cx = AXIS_W + slot * i + slot / 2;
              const x = cx - barW / 2;
              return (
                <g key={b.from} className={`viz-col ${hover === i ? "on" : ""}`}>
                  {stackSegments(b.counts, y(0), H / top, GAP).map((sg) =>
                    sg.top ? (
                      <path key={sg.series} className={`bar ${sg.series}`} d={topRounded(x, sg.y, barW, sg.h, 4)} />
                    ) : (
                      <rect key={sg.series} className={`bar ${sg.series}`} x={x} y={sg.y} width={barW} height={sg.h} />
                    ),
                  )}
                  {showLabel(i) && (
                    <text className="viz-x" x={cx} y={TOP + H + 16} textAnchor="middle">{bucketLabel(b).split("〜")[0]}</text>
                  )}
                  <rect
                    className="viz-hit"
                    x={AXIS_W + slot * i}
                    y={TOP}
                    width={slot}
                    height={H}
                    tabIndex={0}
                    aria-label={describe(b)}
                    onPointerEnter={() => setHover(i)}
                    onFocus={() => setHover(i)}
                    onBlur={() => setHover(null)}
                  />
                </g>
              );
            })}
            <line className="viz-base" x1={AXIS_W} x2={width} y1={y(0)} y2={y(0)} />
          </svg>
        )}
        {hb && hover !== null && (
          <div
            className="viz-tip"
            // 棒を隠さないよう横に出す（右半分の棒なら左側へ）
            style={(() => {
              const cx = AXIS_W + slot * hover + slot / 2;
              const off = barW / 2 + 10;
              return cx > width / 2
                ? { left: cx - off, top: TOP, transform: "translateX(-100%)" }
                : { left: cx + off, top: TOP };
            })()}
            role="status"
          >
            <div className="viz-tip-head">{bucketLabel(hb)}</div>
            {SERIES.map((s) => (
              <div key={s} className="viz-tip-row"><i className={`key ${s}`} /><b>{hb.counts[s]}</b><span>{VERDICT_LABEL[s]}</span></div>
            ))}
          </div>
        )}
      </div>
      <details className="viz-table">
        <summary>表で見る</summary>
        <table>
          <thead><tr><th>{size > 1 ? "週" : "日付"}</th>{SERIES.map((s) => <th key={s} className="num">{VERDICT_LABEL[s]}</th>)}<th className="num">計</th></tr></thead>
          <tbody>
            {buckets.filter((b) => total(b.counts) > 0).reverse().map((b) => (
              <tr key={b.from}><td>{bucketLabel(b)}</td>{SERIES.map((s) => <td key={s} className="num">{b.counts[s]}</td>)}<td className="num">{total(b.counts)}</td></tr>
            ))}
          </tbody>
        </table>
      </details>
    </figure>
  );
}
