import { useMemo, type MouseEvent } from "react";
import { renderMarkdown } from "../markdown";
import { api } from "../api";

/** 解説 Markdown。生 HTML は無効化済み。リンクは OS のブラウザで開く。 */
export function Markdown({ source }: { source: string }) {
  const html = useMemo(() => renderMarkdown(source), [source]);
  const onClick = (e: MouseEvent<HTMLDivElement>) => {
    const a = (e.target as HTMLElement).closest("a");
    if (!a) return;
    e.preventDefault();
    const href = a.getAttribute("href");
    if (href) void api.openExternal(href);
  };
  return <div className="prose" onClick={onClick} dangerouslySetInnerHTML={{ __html: html }} />;
}
