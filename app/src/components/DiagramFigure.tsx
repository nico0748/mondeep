import { useEffect, useMemo, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { KIND_LABEL, renderDiagram, toSvgString, type Offsets } from "../zukai";
import { api } from "../api";
import { useLoad } from "../hooks";
import { svgToReact } from "./SvgView";
import { base64FromString } from "../encode";
import { safeFileName } from "../obsidian";

async function svgToPngBase64(svg: string, w: number, h: number, scale = 2): Promise<string> {
  const img = new Image();
  img.src = `data:image/svg+xml;base64,${base64FromString(svg)}`;
  await img.decode();
  const canvas = document.createElement("canvas");
  canvas.width = w * scale;
  canvas.height = h * scale;
  const ctx = canvas.getContext("2d")!;
  ctx.scale(scale, scale);
  ctx.drawImage(img, 0, 0, w, h);
  return canvas.toDataURL("image/png").split(",")[1];
}

interface Drag {
  id: string;
  pointerId: number;
  startX: number;
  startY: number;
  base: { dx: number; dy: number };
  last: { dx: number; dy: number } | null;
  scale: number;
}

/**
 * 図 1 つ。クリックで拡大、SVG / PNG で書き出し。
 * 概念図と状態遷移図は、拡大画面で部品をドラッグして位置を微調整できる。
 * 微調整は「用語 × 図の種類 × 部品 ID」で保存するので、Claude が描き直しても同じ部品には引き継がれる。
 */
export function DiagramFigure({ data, name, termId }: { data: unknown; name: string; termId: number }) {
  const kind = (data as { kind?: string })?.kind ?? "";
  const title = (data as { title?: string })?.title;
  const [open, setOpen] = useState(false);
  const [editing, setEditing] = useState(false);
  const [note, setNote] = useState<string | null>(null);
  const saved = useLoad(() => api.diagramAdjustments(termId), [termId]);
  const [offsets, setOffsets] = useState<Offsets>({});
  const drag = useRef<Drag | null>(null);

  // 保存済みの微調整をこの図の種類のぶんだけ読み込む（ドラッグ中は上書きしない）
  useEffect(() => {
    if (drag.current || !saved.data) return;
    const o: Offsets = {};
    for (const a of saved.data) if (a.kind === kind) o[a.element_id] = { dx: a.dx, dy: a.dy };
    setOffsets(o);
  }, [saved.data, kind]);

  const rendered = useMemo(() => {
    try {
      return { ok: true as const, r: renderDiagram(data, { offsets }) };
    } catch (e) {
      return { ok: false as const, msg: String(e instanceof Error ? e.message : e) };
    }
  }, [data, offsets]);

  if (!rendered.ok) {
    return <div className="diagram-error">この図は描けませんでした: {rendered.msg}</div>;
  }
  const { r } = rendered;
  const adjusted = Object.keys(offsets).length > 0;

  const exportAs = async (ext: "svg" | "png") => {
    try {
      const svg = toSvgString(r.svg);
      const b64 = ext === "svg" ? base64FromString(svg) : await svgToPngBase64(svg, r.width, r.height);
      const path = await api.exportFile(`${safeFileName(name, 100)}.${ext}`, b64);
      setNote(`保存しました: ${path}`);
    } catch (e) {
      setNote(`書き出せませんでした: ${e}`);
    }
  };

  const onPointerDown = (e: ReactPointerEvent<HTMLDivElement>) => {
    if (!editing) return;
    const node = (e.target as Element).closest("g.zk-node[data-id]");
    const svg = e.currentTarget.querySelector("svg");
    if (!node || !svg) return;
    const id = node.getAttribute("data-id")!;
    const box = svg.getBoundingClientRect();
    e.currentTarget.setPointerCapture(e.pointerId);
    drag.current = {
      id,
      pointerId: e.pointerId,
      startX: e.clientX,
      startY: e.clientY,
      base: offsets[id] ?? { dx: 0, dy: 0 },
      last: null,
      scale: r.width / box.width,
    };
    e.preventDefault();
  };
  const onPointerMove = (e: ReactPointerEvent<HTMLDivElement>) => {
    const d = drag.current;
    if (!d || d.pointerId !== e.pointerId) return;
    const dx = Math.round(d.base.dx + (e.clientX - d.startX) * d.scale);
    const dy = Math.round(d.base.dy + (e.clientY - d.startY) * d.scale);
    d.last = { dx, dy };
    setOffsets((o) => ({ ...o, [d.id]: { dx, dy } }));
  };
  const onPointerUp = async (e: ReactPointerEvent<HTMLDivElement>) => {
    const d = drag.current;
    if (!d || d.pointerId !== e.pointerId) return;
    drag.current = null;
    const o = d.last;
    if (!o) return; // 動かさずに離した
    try {
      await api.setDiagramAdjustment(termId, kind, d.id, o.dx, o.dy);
      setNote(null);
    } catch (err) {
      setNote(`位置を保存できませんでした: ${err}`);
    }
  };
  const reset = async () => {
    await api.resetDiagramAdjustments(termId, kind);
    setOffsets({});
    saved.reload();
    setNote("自動配置に戻しました。");
  };

  return (
    <figure className="diagram">
      <button className="diagram-canvas" onClick={() => setOpen(true)} aria-label={`${title ?? KIND_LABEL[kind]}を拡大`}>
        <span className="diagram-fit" style={{ maxWidth: Math.round(r.width * 1.15) }}>
          {svgToReact({ ...r.svg, attrs: { ...r.svg.attrs, width: "100%", height: "auto" } })}
        </span>
      </button>
      <figcaption>
        <span>
          {KIND_LABEL[kind]}{title ? `：${title}` : ""}
          {adjusted && <span className="adjusted">位置を調整済み</span>}
        </span>
        <span className="diagram-actions">
          <button className="link" onClick={() => exportAs("svg")}>SVG で保存</button>
          <button className="link" onClick={() => exportAs("png")}>PNG で保存</button>
        </span>
      </figcaption>
      {note && <p className="note">{note}</p>}
      {open && (
        <div className="modal" role="dialog" aria-modal="true" aria-label={title ?? KIND_LABEL[kind]} onClick={() => !drag.current && (setOpen(false), setEditing(false))}>
          <div className="modal-body" onClick={(e) => e.stopPropagation()}>
            <div className="modal-bar">
              {r.adjustable && (
                <>
                  <button className={`toggle ${editing ? "on" : ""}`} aria-pressed={editing} onClick={() => setEditing((v) => !v)}>
                    {editing ? "調整を終える" : "位置を調整"}
                  </button>
                  {adjusted && <button onClick={reset}>自動配置に戻す</button>}
                  {editing && <span className="hint-inline">部品をドラッグすると、部品ごとに位置を保存します</span>}
                </>
              )}
              <button className="modal-close" onClick={() => (setOpen(false), setEditing(false))}>閉じる</button>
            </div>
            <div
              className={`modal-canvas ${editing ? "editing" : ""}`}
              onPointerDown={onPointerDown}
              onPointerMove={onPointerMove}
              onPointerUp={onPointerUp}
              onPointerCancel={onPointerUp}
            >
              {svgToReact(r.svg)}
            </div>
          </div>
        </div>
      )}
    </figure>
  );
}
