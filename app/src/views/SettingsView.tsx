import { useState } from "react";
import { api } from "../api";
import { useLoad } from "../hooks";
import { ErrorNote } from "../components/bits";

export function SettingsView() {
  const { data, error } = useLoad(() => api.appInfo(), []);
  const [note, setNote] = useState<string | null>(null);
  if (error) return <section className="view"><ErrorNote error={error} /></section>;
  if (!data) return <section className="view" aria-busy="true" />;

  const copy = async () => {
    await navigator.clipboard.writeText(data.mcp_config_snippet);
    setNote("コピーしました。Claude Desktop の設定ファイルの mcpServers に貼り付けてください。");
  };
  const install = async () => {
    if (!window.confirm(`${data.claude_config_path} に Mondeep の登録を追記します。元のファイルは .bak として残します。`)) return;
    try {
      const path = await api.installMcpConfig();
      setNote(`${path} に追記しました。Claude Desktop を再起動すると使えるようになります。`);
    } catch (e) {
      setNote(`追記できませんでした: ${e}`);
    }
  };

  return (
    <section className="view settings">
      <header className="view-head"><h1>設定</h1></header>
      <div className="block">
        <h2>Claude Desktop への登録</h2>
        <p>Claude Desktop が Mondeep の MCP サーバーを起動し、会話の記録をこのアプリの DB に保存します。</p>
        {!data.mcp_binary_exists && (
          <p className="error">MCP サーバーの実行ファイルが見つかりません: {data.mcp_binary}。<code>cargo build --release -p mondeep-mcp</code> でビルドし、アプリと同じフォルダに置いてください。</p>
        )}
        <pre className="code">{data.mcp_config_snippet}</pre>
        <div className="actions">
          <button onClick={copy}>設定をコピー</button>
          <button onClick={install} disabled={!data.claude_config_path}>設定ファイルに追記</button>
        </div>
        {note && <p className="note">{note}</p>}
      </div>
      <div className="block">
        <h2>データの保存場所</h2>
        <dl className="kv">
          <dt>学習 DB</dt><dd><code>{data.db_path}</code></dd>
          <dt>バックアップ（起動時・直近 7 世代）</dt><dd><code>{data.backup_dir}</code></dd>
          <dt>Claude Desktop の設定ファイル</dt><dd><code>{data.claude_config_path ?? "不明"}</code></dd>
        </dl>
        {data.backup_error && <p className="error">起動時のバックアップに失敗しました: {data.backup_error}</p>}
        <p className="hint">学習記録はこの端末の外に送られません。アプリと MCP サーバーは外部と通信しません。</p>
      </div>
      <p className="meta">Mondeep {data.version}</p>
    </section>
  );
}
