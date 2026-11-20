import { useMemo, useState } from "react";
import { KIND_LABEL, renderDiagram, toSvgString } from "../zukai";
import { api } from "../api";
import { svgToReact } from "./SvgView";

function base64FromString(s: string): string {
  const bytes = new TextEncoder().encode(s);
  let bin = "";
  bytes.forEach((b) => (bin += String.fromCharCode(b)));
  return btoa(bin);
}

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

/** 図 1 つ。クリックで拡大、SVG / PNG で書き出し。 */
export function DiagramFigure({ data, name }: { data: unknown; name: string }) {
  const [open, setOpen] = useState(false);
  const [note, setNote] = useState<string | null>(null);
  const rendered = useMemo(() => {
    try {
      return { ok: true as const, r: renderDiagram(data) };
    } catch (e) {
      return { ok: false as const, msg: String(e instanceof Error ? e.message : e) };
    }
  }, [data]);

  if (!rendered.ok) {
    return <div className="diagram-error">この図は描けませんでした: {rendered.msg}</div>;
  }
  const { r } = rendered;
  const kind = (data as { kind: string }).kind;
  const title = (data as { title?: string }).title;

  const exportAs = async (ext: "svg" | "png") => {
    try {
      const svg = toSvgString(r.svg);
      const b64 = ext === "svg" ? base64FromString(svg) : await svgToPngBase64(svg, r.width, r.height);
      const path = await api.exportFile(`${name}.${ext}`, b64);
      setNote(`保存しました: ${path}`);
    } catch (e) {
      setNote(`書き出せませんでした: ${e}`);
    }
  };

  return (
    <figure className="diagram">
      <button className="diagram-canvas" onClick={() => setOpen(true)} aria-label={`${title ?? KIND_LABEL[kind]}を拡大`}>
        <span className="diagram-fit" style={{ maxWidth: Math.round(r.width * 1.15) }}>
          {svgToReact({ ...r.svg, attrs: { ...r.svg.attrs, width: "100%", height: "auto" } })}
        </span>
      </button>
      <figcaption>
        <span>{KIND_LABEL[kind]}{title ? `：${title}` : ""}</span>
        <span className="diagram-actions">
          <button className="link" onClick={() => exportAs("svg")}>SVG で保存</button>
          <button className="link" onClick={() => exportAs("png")}>PNG で保存</button>
        </span>
      </figcaption>
      {note && <p className="note">{note}</p>}
      {open && (
        <div className="modal" role="dialog" aria-modal="true" onClick={() => setOpen(false)}>
          <div className="modal-body" onClick={(e) => e.stopPropagation()}>
            <button className="modal-close" onClick={() => setOpen(false)}>閉じる</button>
            <div className="modal-canvas">{svgToReact(r.svg)}</div>
          </div>
        </div>
      )}
    </figure>
  );
}
