import { describe, expect, it } from "vitest";
import { bucketize, bucketLabel, bucketSize, levelLabel, niceTicks, stackSegments } from "./stats";

const day = (date: string, correct = 0, partial = 0, incorrect = 0) => ({ date, counts: { correct, partial, incorrect } });

describe("学習統計の計算", () => {
  it("日別の行を、今日で終わる区切りにまとめる", () => {
    const daily = Array.from({ length: 10 }, (_, i) => day(`2026-11-${String(11 + i).padStart(2, "0")}`, 1, i % 2, 0));
    const b = bucketize(daily, 7);
    expect(b).toHaveLength(2);
    expect(b[1]).toMatchObject({ from: "2026-11-14", to: "2026-11-20", counts: { correct: 7, partial: 4, incorrect: 0 } });
    expect(b[0]).toMatchObject({ from: "2026-11-11", to: "2026-11-13" });
    expect(bucketLabel(b[1])).toBe("11/14〜11/20");
    expect(bucketize(daily, 1)).toHaveLength(10);
    expect(bucketLabel(bucketize(daily, 1)[0])).toBe("11/11");
  });

  it("期間が長いときだけ週ごとにする", () => {
    expect(bucketSize(30)).toBe(1);
    expect(bucketSize(90)).toBe(1);
    expect(bucketSize(365)).toBe(7);
  });

  it("目盛りはきりのよい数で最大値を超える", () => {
    expect(niceTicks(0)).toEqual([0, 1]);
    expect(niceTicks(3)).toEqual([0, 1, 2, 3]);
    expect(niceTicks(17)).toEqual([0, 5, 10, 15, 20]);
    expect(niceTicks(120)).toEqual([0, 50, 100, 150]);
  });

  it("積み上げの段は、大きな週と同じ目盛りでも少ない判定を消さない", () => {
    // 目盛りの上限 150 件を高さ 200px で描くと、1 件は約 1.33px
    const segs = stackSegments({ correct: 1, partial: 0, incorrect: 1 }, 208, 200 / 150);
    expect(segs.map((s) => s.series)).toEqual(["correct", "incorrect"]);
    for (const s of segs) expect(s.h).toBeGreaterThanOrEqual(1);
    // 下の段の上端はすき間を空けて、上の段の下端より下にある
    expect(segs[0].y).toBeGreaterThanOrEqual(segs[1].y + segs[1].h);
    // 十分高い段は 2px のすき間を空ける
    const big = stackSegments({ correct: 10, partial: 5, incorrect: 0 }, 200, 10);
    expect(big[0]).toMatchObject({ series: "correct", h: 98, y: 100, top: false });
    expect(big[1]).toMatchObject({ series: "partial", h: 50, y: 50, top: true });
  });

  it("段数の区分はアプリの呼び方（最初の問い・1 段目…）に合わせる", () => {
    expect([0, 1, 2, 4].map((l) => levelLabel(l, l === 4))).toEqual(["正解なし", "最初の問い", "1 段目", "3 段目以上"]);
  });
});
