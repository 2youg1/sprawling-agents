// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { PDFDocumentProxy } from "pdfjs-dist";
import { createEffect, createSignal, onCleanup, Show } from "solid-js";

import { fill, say } from "../../../core/lang";
import type { Lang } from "../../../core/lang";
import { drawPdf, type PdfDetail, type Size } from "./pdf";
import { regionStyle, type PdfRegion } from "./pdf_anchor";
import { releaseRaster, type PdfRaster, type PdfRasterCache } from "./pdf_raster_cache";
import type { RasterStage } from "./pdf_window";

interface PageProps {
  readonly pdf: PDFDocumentProxy;
  readonly detailDocument: PdfDetail;
  readonly page: number;
  readonly width: number;
  readonly lang: Lang;
  readonly measured?: (size: Size) => void;
  readonly selectable?: boolean;
  readonly detail?: boolean;
  readonly prepare?: boolean;
  readonly onReady?: (width: number, stage: RasterStage) => void;
  readonly cache?: PdfRasterCache;
}

function enhanced(bitmap: ImageBitmap): HTMLCanvasElement {
  const canvas = document.createElement("canvas");
  canvas.width = bitmap.width;
  canvas.height = bitmap.height;
  const context = canvas.getContext("bitmaprenderer");
  if (context === null) canvas.getContext("2d")?.drawImage(bitmap, 0, 0);
  else context.transferFromImageBitmap(bitmap);
  bitmap.close();
  return canvas;
}

export function PdfPage(props: PageProps) {
  let pixels: HTMLDivElement | undefined;
  let words: HTMLDivElement | undefined;
  let visible: { page: number; width: number; raster: PdfRaster } | undefined;
  const [failed, setFailed] = createSignal<string>();
  const [painter, setPainter] = createSignal<ReturnType<typeof drawPdf>>();
  const detailed = () => props.detail !== false;

  function release(): void {
    if (visible === undefined) return;
    const { page, width, raster } = visible;
    visible = undefined;
    if (props.cache === undefined) releaseRaster(raster);
    else props.cache.keep(page, width, raster);
  }

  createEffect(
    () => ({
      pdf: props.pdf,
      number: props.page,
      width: props.width,
      selectable: props.selectable,
    }),
    ({ pdf, number, width, selectable }) => {
      const surface = pixels;
      if (surface === undefined) return;
      setFailed(undefined);
      const display = (raster: PdfRaster): void => {
        setFailed(undefined);
        if (visible?.page === number && visible.width === width) {
          releaseRaster(visible.raster);
        } else release();
        visible = { page: number, width, raster };
        surface.replaceChildren(raster.original);
        if (raster.clear !== null) surface.append(raster.clear);
        props.onReady?.(width, raster.full ? "full" : "preview");
      };
      const cameBack = props.cache?.take(number, width);
      if (cameBack !== undefined) display(cameBack);
      const cached =
        visible?.page === number && visible.width === width ? visible.raster : undefined;
      const paint = drawPdf({
        pdf,
        detail: props.detailDocument,
        number,
        width,
        present: (original, tones, full) => {
          original.className = "pdf-original";
          if (full) original.dataset.pdfQuality = "full";
          const clear =
            tones instanceof HTMLCanvasElement || tones === null ? tones : enhanced(tones);
          if (clear !== null) clear.className = "pdf-clear";
          display({ original, full, clear });
        },
        measure: (size) => {
          props.measured?.(size);
        },
        failure: (reason) => {
          setFailed(String(reason));
          props.onReady?.(width, "full");
        },
        ...(cached === undefined ? {} : { cached }),
        ...(words === undefined || selectable === false ? {} : { textLayer: words }),
      });
      setPainter(paint);
      return () => {
        paint.close();
        props.onReady?.(width, "empty");
      };
    },
  );
  createEffect(
    () => ({ paint: painter(), detail: detailed(), prepare: props.prepare }),
    ({ paint, detail, prepare }) => {
      if (prepare === true) paint?.prepare();
      if (detail) paint?.sharpen();
      else paint?.pause();
    },
  );
  onCleanup(() => {
    pixels?.replaceChildren();
    release();
  });

  return (
    <>
      <div
        class="pdf-pixels"
        aria-hidden="true"
        ref={(node) => {
          pixels = node;
        }}
      />
      <Show when={props.selectable !== false}>
        <div
          class="pdf-text-layer"
          ref={(node) => {
            words = node;
          }}
        />
      </Show>
      <Show when={failed()}>
        {(reason) => (
          <p class="pdf-page-failure" role="status">
            {fill(say(props.lang, "format_broken"), { format: "PDF", reason: reason() })}
          </p>
        )}
      </Show>
    </>
  );
}

interface SheetProps extends PageProps {
  readonly size: Size;
  readonly top: number | undefined;
  readonly total: number;
  readonly box: PdfRegion | undefined;
  readonly reference: PdfRegion | undefined;
}

export function PdfSheet(props: SheetProps) {
  return (
    <div
      class="pdf-sheet"
      data-page={String(props.page)}
      style={{
        "aspect-ratio": String(props.size.width) + " / " + String(props.size.height),
        "inset-block-start": props.top === undefined ? undefined : String(props.top) + "px",
      }}
      role="group"
      aria-label={fill(say(props.lang, "format_page"), {
        n: String(props.page),
        total: String(props.total),
      })}
    >
      <Show when={props.width > 0}>
        <PdfPage {...props} />
      </Show>
      <Show when={props.box}>
        {(region) => <div class="pdf-region" style={regionStyle(region())} aria-hidden="true" />}
      </Show>
      <Show when={props.reference}>
        {(region) => (
          <div class="pdf-region pdf-reference" style={regionStyle(region())} aria-hidden="true" />
        )}
      </Show>
    </div>
  );
}
