import { createElement, type ReactNode } from "react";
import type { SvgNode } from "../zukai";

const toReactAttr = (k: string) => {
  if (k === "class") return "className";
  if (k.startsWith("data-") || k.startsWith("aria-")) return k;
  return k.replace(/-([a-z])/g, (_, c: string) => c.toUpperCase());
};

/** 図解フレームワークの要素木を React 要素にする（innerHTML を使わない）。 */
export function svgToReact(node: SvgNode | string, key?: number): ReactNode {
  if (typeof node === "string") return node;
  const props: Record<string, unknown> = { key };
  for (const [k, v] of Object.entries(node.attrs)) props[toReactAttr(k)] = v;
  return createElement(node.tag, props, ...node.children.map((c, i) => svgToReact(c, i)));
}
