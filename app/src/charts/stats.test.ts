import { describe, expect, it } from "vitest";
import { bucketize, bucketLabel, bucketSize, levelLabel, niceTicks } from "./stats";

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

  it("段数の区分はアプリの呼び方（最初の問い・1 段目…）に合わせる", () => {
    expect([0, 1, 2, 4].map((l) => levelLabel(l, l === 4))).toEqual(["正解なし", "最初の問い", "1 段目", "3 段目以上"]);
  });
});
