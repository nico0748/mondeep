// 学習統計の画面で使う計算（描画から切り離してテストできるようにする）。
import type { DailyStat, VerdictCounts } from "../types";

export interface Bucket {
  /** 区切りの最初と最後の日（YYYY-MM-DD） */
  from: string;
  to: string;
  counts: VerdictCounts;
}

export const total = (c: VerdictCounts) => c.correct + c.partial + c.incorrect;

/** 日別の行を、`size` 日ずつにまとめる（期間が長いときは週ごとにして棒を太く保つ）。最後の区切りは今日で終わる。 */
export function bucketize(daily: DailyStat[], size: number): Bucket[] {
  const out: Bucket[] = [];
  const n = Math.max(1, Math.floor(size));
  // 今日の側からまとめるので、端数は最初の区切りに寄る
  for (let end = daily.length; end > 0; end -= n) {
    const slice = daily.slice(Math.max(0, end - n), end);
    const counts = { correct: 0, partial: 0, incorrect: 0 };
    for (const d of slice) {
      counts.correct += d.counts.correct;
      counts.partial += d.counts.partial;
      counts.incorrect += d.counts.incorrect;
    }
    out.unshift({ from: slice[0].date, to: slice[slice.length - 1].date, counts });
  }
  return out;
}

/** 期間に合う区切りの日数（90 日までは 1 日、それより長ければ 1 週）。 */
export const bucketSize = (days: number) => (days > 90 ? 7 : 1);

/** 0 から始まるきりのよい目盛り（1・2・5 の倍数）。最大値が 0 でも 1 本は引く。 */
export function niceTicks(max: number, target = 4): number[] {
  if (max <= 0) return [0, 1];
  const raw = max / target;
  const pow = 10 ** Math.floor(Math.log10(raw));
  const step = [1, 2, 5, 10].map((m) => m * pow).find((s) => s >= raw) ?? 10 * pow;
  const s = Math.max(1, Math.round(step));
  const ticks: number[] = [];
  for (let v = 0; v < max + s; v += s) ticks.push(v);
  return ticks;
}

/** 「11/20」形式。 */
export function shortDate(iso: string): string {
  const [, m, d] = iso.split("-").map(Number);
  return `${m}/${d}`;
}

/** 区切りの名前（1 日なら「11/20」、まとめたら「11/14〜11/20」）。 */
export function bucketLabel(b: Bucket): string {
  return b.from === b.to ? shortDate(b.from) : `${shortDate(b.from)}〜${shortDate(b.to)}`;
}

/** 正解できた段数の区分の名前（段数 = 正解できた最も深い段 + 1、最初の問いが 1 段目）。 */
export function levelLabel(levels: number, last: boolean): string {
  if (levels === 0) return "正解なし";
  if (levels === 1) return "最初の問い";
  return `${levels - 1} 段目${last ? "以上" : ""}`;
}
