// Rust 側（mondeep-core の model.rs）と同じ形の型。
export type Format = "meaning_to_term" | "term_to_meaning";
export type SessionFormat = Format | "mixed";
export type Verdict = "correct" | "partial" | "incorrect";

export interface Term { id: number; headword: string; aliases: string[]; tags: string[]; needs_review: boolean; created_at: string }
export interface Session { id: number; format: SessionFormat; theme: string; started_at: string; ended_at: string | null; summary: string | null; accuracy: number | null }
export interface Question { id: number; session_id: number; term_id: number; format: Format; prompt: string; model_answer: string | null; parent_id: number | null; depth: number; created_at: string }
export interface QuestionSummary extends Question { headword: string; latest_verdict: Verdict | null; attempt_count: number }
export interface Attempt { id: number; question_id: number; answer: string; verdict: Verdict; reason: string; answered_at: string }
export interface Source { kind: "obsidian" | "knowledge_db" | "web" | "other"; name: string; url?: string }
export interface Explanation { id: number; question_id: number; attempt_id: number | null; body_md: string; diagrams: unknown[]; sources: Source[]; created_at: string }
export interface QuestionDetail { question: Question; term: Term; session: Session; attempts: Attempt[]; explanation: Explanation | null; tree: QuestionSummary[]; root_id: number }
export interface SessionListItem extends Session { question_count: number; correct: number; partial: number; incorrect: number; tags: string[] }
export interface HistoryGroup { session: SessionListItem; questions: QuestionSummary[] }
export interface VerdictCounts { correct: number; partial: number; incorrect: number }
export interface ReviewState { term_id: number; format: Format; due_date: string; interval_days: number; ease: number; streak: number; last_reviewed_at: string | null }
export interface TermListItem extends Term { counts: VerdictCounts; accuracy: number | null; last_asked_at: string | null; next_due: string | null }
export interface TermCard { term: Term; counts: VerdictCounts; accuracy: number | null; review_states: ReviewState[]; questions: QuestionSummary[]; latest_explanation: Explanation | null; max_correct_depth: number | null }
export interface ReviewItem { term_id: number; headword: string; aliases: string[]; tags: string[]; format: Format | null; due_date: string | null; needs_review: boolean; counts: VerdictCounts; last_verdict: Verdict | null; summary: string }
export interface WeakTerm { term_id: number; headword: string; tags: string[]; needs_review: boolean; counts: VerdictCounts; accuracy: number | null; avg_max_correct_depth: number | null; score: number; reasons: string[] }
export interface SearchHit { kind: "term" | "question" | "explanation"; term_id: number; headword: string; question_id: number | null; snippet: string }
export interface HistoryFilter { format?: Format; tag?: string; verdict?: Verdict; limit?: number; offset?: number }
export interface AppInfo { version: string; db_path: string; backup_dir: string; backup_error: string | null; mcp_binary: string; mcp_binary_exists: boolean; mcp_config_snippet: string; claude_config_path: string | null; today: string }

export const FORMAT_LABEL: Record<SessionFormat, string> = { meaning_to_term: "意味→用語", term_to_meaning: "用語→意味＋深掘り", mixed: "混合" };
export const VERDICT_LABEL: Record<Verdict, string> = { correct: "正解", partial: "部分正解", incorrect: "不正解" };
export const SOURCE_LABEL: Record<Source["kind"], string> = { obsidian: "Obsidian", knowledge_db: "ナレッジ DB", web: "Web", other: "その他" };
export interface DiagramAdjustment { term_id: number; kind: string; element_id: string; dx: number; dy: number; updated_at: string }
