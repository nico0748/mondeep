import { createContext, useContext, useEffect, useState } from "react";

/** DB の更新回数。MCP サーバーが書き込むと増え、各画面が読み直す。 */
export const RevContext = createContext(0);

export function useLoad<T>(load: () => Promise<T>, deps: unknown[]): { data: T | null; error: string | null; reload: () => void } {
  const rev = useContext(RevContext);
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [tick, setTick] = useState(0);
  useEffect(() => {
    let alive = true;
    load()
      .then((d) => alive && (setData(d), setError(null)))
      .catch((e) => alive && setError(String(e)));
    return () => {
      alive = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [...deps, rev, tick]);
  return { data, error, reload: () => setTick((t) => t + 1) };
}

/** ハッシュでの画面遷移（#/term/12 など）。 */
export function useRoute(): string[] {
  const [hash, setHash] = useState(() => window.location.hash);
  useEffect(() => {
    const on = () => setHash(window.location.hash);
    window.addEventListener("hashchange", on);
    return () => window.removeEventListener("hashchange", on);
  }, []);
  const path = hash.replace(/^#\/?/, "").split("?")[0];
  return path ? path.split("/").map(decodeURIComponent) : ["history"];
}

export function go(path: string) {
  window.location.hash = `#/${path}`;
}
