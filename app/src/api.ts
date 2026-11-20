// Tauri コマンドの呼び出し。画面から呼べる操作は src-tauri/capabilities/default.json で許可したものだけ。
import { invoke } from "@tauri-apps/api/core";
import type { AppInfo, Attempt, DiagramAdjustment, ExportFile, HistoryFilter, HistoryGroup, Question, QuestionDetail, ReviewItem, SearchHit, Stats, TagStat, Term, TermCard, TermExport, TermListItem, Verdict, WeakTerm } from "./types";

export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),
  dataVersion: () => invoke<number>("data_version"),
  history: (filter: HistoryFilter) => invoke<HistoryGroup[]>("history", { filter }),
  questionDetail: (id: number) => invoke<QuestionDetail>("question_detail", { id }),
  listTerms: (tag?: string, query?: string) => invoke<TermListItem[]>("list_terms", { tag: tag || null, query: query || null }),
  termCard: (id: number) => invoke<TermCard>("term_card", { id }),
  allTags: () => invoke<string[]>("all_tags"),
  reviewQueue: () => invoke<ReviewItem[]>("review_queue"),
  weakTerms: () => invoke<WeakTerm[]>("weak_terms"),
  search: (keyword: string) => invoke<SearchHit[]>("search", { keyword }),
  setNeedsReview: (termId: number, on: boolean) => invoke<Term>("set_needs_review", { termId, on }),
  setTermTags: (termId: number, tags: string[]) => invoke<Term>("set_term_tags", { termId, tags }),
  diagramAdjustments: (termId: number) => invoke<DiagramAdjustment[]>("diagram_adjustments", { termId }),
  setDiagramAdjustment: (termId: number, kind: string, elementId: string, dx: number, dy: number) =>
    invoke<void>("set_diagram_adjustment", { termId, kind, elementId, dx, dy }),
  resetDiagramAdjustments: (termId: number, kind: string) => invoke<number>("reset_diagram_adjustments", { termId, kind }),
  deleteSession: (id: number) => invoke<void>("delete_session", { id }),
  installMcpConfig: () => invoke<string>("install_mcp_config"),
  openExternal: (url: string) => invoke<void>("open_external", { url }),
  exportFile: (fileName: string, dataBase64: string) => invoke<string>("export_file", { fileName, dataBase64 }),
  exportFiles: (folder: string, files: ExportFile[]) => invoke<string>("export_files", { folder, files }),
  backupNow: () => invoke<string>("backup_now"),
  overrideVerdict: (attemptId: number, verdict: Verdict | null) => invoke<Attempt>("override_verdict", { attemptId, verdict }),
  setQuestionNote: (questionId: number, note: string) => invoke<Question>("set_question_note", { questionId, note }),
  tagStats: () => invoke<TagStat[]>("tag_stats"),
  renameTag: (old: string, newName: string) => invoke<number>("rename_tag", { old, new: newName }),
  removeTag: (tag: string) => invoke<number>("remove_tag", { tag }),
  stats: (days: number) => invoke<Stats>("stats", { days }),
  termExport: (id: number) => invoke<TermExport>("term_export", { id }),
  exportAll: () => invoke<TermExport[]>("export_all"),
};
