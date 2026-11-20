/** UTF-8 の文字列を base64 にする（Tauri コマンドでファイルを渡すとき用）。 */
export function base64FromString(s: string): string {
  const bytes = new TextEncoder().encode(s);
  let bin = "";
  const CHUNK = 0x8000;
  for (let i = 0; i < bytes.length; i += CHUNK) {
    bin += String.fromCharCode(...bytes.subarray(i, i + CHUNK));
  }
  return btoa(bin);
}
