// 解説 Markdown の描画。生 HTML は無効、画像は読み込まない（外部通信をしない）、リンクは http(s) だけ。
import MarkdownIt from "markdown-it";

const md = new MarkdownIt({ html: false, linkify: true, typographer: false, breaks: false });
md.disable(["image"]);

const SAFE_URL = /^https?:\/\//i;
md.validateLink = (url: string) => SAFE_URL.test(url.trim());

const defaultLinkOpen = md.renderer.rules.link_open ?? ((tokens, idx, options, _env, self) => self.renderToken(tokens, idx, options));
md.renderer.rules.link_open = (tokens, idx, options, env, self) => {
  const t = tokens[idx];
  // クリックは画面側で拾って OS のブラウザで開く
  t.attrSet("data-external", "true");
  t.attrSet("rel", "noreferrer noopener");
  return defaultLinkOpen(tokens, idx, options, env, self);
};

export function renderMarkdown(src: string): string {
  return md.render(src);
}
