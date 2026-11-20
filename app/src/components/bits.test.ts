import { describe, expect, it } from "vitest";
import { orderTree } from "./bits";
import type { QuestionSummary } from "../types";

const q = (id: number, parent: number | null, depth: number): QuestionSummary => ({
  id, parent_id: parent, depth, session_id: 1, term_id: 1, format: "term_to_meaning", prompt: `q${id}`,
  model_answer: null, created_at: "", note: null, headword: "XSS", latest_verdict: null, attempt_count: 0,
});

describe("深掘りの木の並び", () => {
  it("親のすぐ後に子が来る", () => {
    const ordered = orderTree([q(1, null, 0), q(4, null, 0), q(2, 1, 1), q(5, 4, 1), q(3, 2, 2)]);
    expect(ordered.map((x) => x.id)).toEqual([1, 2, 3, 4, 5]);
  });
});
