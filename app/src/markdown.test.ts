import { describe, expect, it } from "vitest";
import { renderMarkdown } from "./markdown";

describe("解説 Markdown の描画", () => {
  it("生 HTML は文字として表示する", () => {
    const html = renderMarkdown('<img src=x onerror="alert(1)"><script>alert(1)</script>');
    expect(html).not.toContain("<script>");
    expect(html).not.toContain("<img");
    expect(html).toContain("&lt;script&gt;");
  });
  it("javascript: リンクと画像は描かない", () => {
    const html = renderMarkdown("[x](javascript:alert(1)) ![img](https://example.com/a.png)");
    expect(html).not.toContain('href="javascript');
    expect(html).not.toContain("<img");
  });
  it("http(s) リンクは外部リンクとして印を付ける", () => {
    const html = renderMarkdown("[RFC 8446](https://www.rfc-editor.org/rfc/rfc8446)");
    expect(html).toContain('data-external="true"');
    expect(html).toContain('href="https://www.rfc-editor.org/rfc/rfc8446"');
  });
});
