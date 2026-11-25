// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { OPS, RenderingCancelledException, TextLayer } from "pdfjs-dist";
import { Result } from "effect";
import type { PDFDocumentProxy, PDFPageProxy } from "pdfjs-dist";
import { cropPdf, type PdfRegion } from "./pdf_anchor";
import type { PdfDetail } from "./pdf_document";
import { neutralTones } from "./pdf_tones";

export { gpuAvailability, openPdf, type Opened, type PdfDetail } from "./pdf_document";

export interface Size {
  readonly width: number;
  readonly height: number;
}

const MAX_PIXELS = 4_194_304;
const MAX_SIDE = 4096;

export interface PagePaint {
  readonly pdf: PDFDocumentProxy;
  readonly detail?: PdfDetail;
  readonly number: number;
  readonly width: number;
  readonly present: (
    original: HTMLCanvasElement,
    tones: HTMLCanvasElement | ImageBitmap | null,
    full: boolean,
  ) => void;
  readonly measure: (size: Size) => void;
  readonly failure: (reason: unknown) => void;
  readonly textLayer?: HTMLDivElement;
  readonly cached?: { readonly full: boolean };
}

const PREVIEW_DENSITY = 0.38;
const PREVIEW_PIXELS = 131_072;

function raster(page: PDFPageProxy, width: number, density: number, pixels = MAX_PIXELS) {
  const natural = page.getViewport({ scale: 1 });
  const scale = Math.min(
    (width * density) / natural.width,
    Math.sqrt(pixels / (natural.width * natural.height)),
    MAX_SIDE / Math.max(natural.width, natural.height),
  );
  return page.getViewport({ scale });
}

export function drawPdf(options: PagePaint) {
  const abort = new AbortController();
  const signal = abort.signal;
  const failed = (reason: unknown): null => {
    if (!signal.aborted && !(reason instanceof RenderingCancelledException)) {
      options.failure(reason);
    }
    return null;
  };
  let preview: Promise<PDFPageProxy | null> | undefined;
  let detail: AbortController | undefined;
  let complete = false;
  let textStarted = false;
  function start(): Promise<PDFPageProxy | null> {
    preview ??= paintPreview(options, signal).catch(failed);
    return preview;
  }
  function pause(): void {
    detail?.abort();
    detail = undefined;
  }
  function text(page: PDFPageProxy): void {
    if (textStarted || options.textLayer === undefined) return;
    textStarted = true;
    paintText(page, options.width, options.textLayer, signal, options.failure);
  }
  return {
    close: () => {
      pause();
      abort.abort();
    },
    prepare: () => {
      void start();
    },
    pause,
    sharpen: () => {
      if (complete || detail !== undefined) return;
      const current = new AbortController();
      detail = current;
      void start()
        .then(async (page) => {
          if (current.signal.aborted) return;
          if (page !== null) text(page);
          await paintDetail(page, options, current.signal, text);
          if (!current.signal.aborted) complete = true;
        })
        .catch((reason: unknown) => {
          if (detail === current && !current.signal.aborted) failed(reason);
        })
        .finally(() => {
          if (detail === current) detail = undefined;
        });
    },
  };
}

function scanned({
  fnArray,
  argsArray,
}: Awaited<ReturnType<PDFPageProxy["getOperatorList"]>>): boolean {
  let textMode = 0;
  let image = false;
  const modeStack: number[] = [];
  for (const [index, instruction] of fnArray.entries()) {
    if (instruction === OPS.save || instruction === OPS.paintFormXObjectBegin) {
      modeStack.push(textMode);
    } else if (instruction === OPS.restore || instruction === OPS.paintFormXObjectEnd) {
      textMode = modeStack.pop() ?? 0;
    } else if (instruction === OPS.setTextRenderingMode) {
      const arguments_: unknown = argsArray[index];
      if (Array.isArray(arguments_) && typeof arguments_[0] === "number") {
        textMode = arguments_[0];
      }
    } else if (instruction === OPS.showText && (textMode & 3) !== 3) {
      return false;
    } else if (
      instruction === OPS.paintImageXObject ||
      instruction === OPS.paintImageMaskXObject ||
      instruction === OPS.paintImageMaskXObjectGroup ||
      instruction === OPS.paintImageXObjectRepeat ||
      instruction === OPS.paintInlineImageXObject ||
      instruction === OPS.paintInlineImageXObjectGroup
    ) {
      image = true;
    }
  }
  return image;
}

function cancelled(signal: AbortSignal): boolean {
  return signal.aborted;
}

async function paintPreview(options: PagePaint, signal: AbortSignal): Promise<PDFPageProxy | null> {
  const page = await options.pdf.getPage(options.number);
  if (cancelled(signal)) return null;
  signal.addEventListener(
    "abort",
    () => {
      page.cleanup();
    },
    { once: true },
  );
  const natural = page.getViewport({ scale: 1 });
  options.measure({ width: natural.width, height: natural.height });
  if (options.cached === undefined) {
    const preview = raster(page, options.width, PREVIEW_DENSITY, PREVIEW_PIXELS);
    const first = await renderRaster(page, preview, signal, true).catch(async (reason: unknown) => {
      if (signal.aborted || options.detail === undefined) return Promise.reject(reason);
      const document = await options.detail();
      if (document === options.pdf) return Promise.reject(reason);
      const original = await document.getPage(options.number);
      return renderRaster(original, preview, signal, true).finally(() => original.cleanup());
    });
    if (first === null) return null;
    options.present(first, Result.getOrNull(Result.try(() => previewTones(first))), false);
  }
  return cancelled(signal) ? null : page;
}

async function paintDetail(
  page: PDFPageProxy | null,
  options: PagePaint,
  signal: AbortSignal,
  text: (page: PDFPageProxy) => void,
): Promise<void> {
  if (options.cached?.full === true) return;
  const document = await options.detail?.();
  if (signal.aborted) return;
  const source = document === undefined ? page : await document.getPage(options.number);
  if (source === null) return;
  if (!signal.aborted) text(source);
  if (source === page) return paintSharp(source, options, signal);
  return paintSharp(source, options, signal).finally(() => {
    source.cleanup();
  });
}

async function paintSharp(
  source: PDFPageProxy,
  options: PagePaint,
  signal: AbortSignal,
): Promise<void> {
  const display = fullViewport(source, options.width);
  const original = await renderRaster(source, display, signal);
  if (original === null) return;
  const operators = await source.getOperatorList().catch(() => null);
  await showTones(
    original,
    operators !== null && scanned(operators),
    options.present,
    true,
    signal,
  );
}

function fullViewport(page: PDFPageProxy, width: number) {
  return raster(page, width, Math.max(1.35, window.devicePixelRatio));
}

export async function cropFullPage(
  detail: PdfDetail,
  width: number,
  picked: { readonly page: number; readonly region: PdfRegion },
  signal: AbortSignal,
): Promise<Blob | null> {
  const pdf = await detail();
  if (signal.aborted) return null;
  const page = await pdf.getPage(picked.page);
  return renderRaster(page, fullViewport(page, width), signal)
    .then((canvas) => {
      if (canvas === null) return null;
      return cropPdf(canvas, picked.region).finally(() => {
        canvas.width = canvas.height = 0;
      });
    })
    .finally(() => page.cleanup());
}

const layers = new WeakMap<HTMLDivElement, { page: PDFPageProxy; layer: TextLayer }>();

function paintText(
  page: PDFPageProxy,
  width: number,
  words: HTMLDivElement,
  signal: AbortSignal,
  failure: (reason: unknown) => void,
): void {
  const natural = page.getViewport({ scale: 1 });
  const viewport = page.getViewport({ scale: width / natural.width });
  words.style.setProperty("--total-scale-factor", String(viewport.scale * viewport.userUnit));
  const prior = layers.get(words);
  if (prior?.page === page) {
    prior.layer.update({ viewport });
    return;
  }
  words.replaceChildren();
  const layer = new TextLayer({
    textContentSource: page.streamTextContent(),
    container: words,
    viewport,
  });
  const cancel = () => {
    layers.delete(words);
    layer.cancel();
  };
  signal.addEventListener("abort", cancel, { once: true });
  void layer
    .render()
    .then(() => {
      if (!cancelled(signal)) layers.set(words, { page, layer });
    })
    .catch((reason: unknown) => {
      if (!cancelled(signal)) failure(reason);
    })
    .finally(() => {
      signal.removeEventListener("abort", cancel);
    });
}

async function renderRaster(
  page: PDFPageProxy,
  viewport: ReturnType<PDFPageProxy["getViewport"]>,
  signal: AbortSignal,
  readback = false,
): Promise<HTMLCanvasElement | null> {
  if (signal.aborted) return null;
  const canvas = document.createElement("canvas");
  canvas.width = Math.max(1, Math.floor(viewport.width));
  canvas.height = Math.max(1, Math.floor(viewport.height));
  const context = readback
    ? canvas.getContext("2d", { alpha: false, willReadFrequently: true })
    : null;
  const task = page.render(
    context === null ? { canvas, viewport } : { canvas, canvasContext: context, viewport },
  );
  const cancel = () => {
    task.cancel();
  };
  signal.addEventListener("abort", cancel, { once: true });
  await task.promise.finally(() => {
    signal.removeEventListener("abort", cancel);
    if (signal.aborted) {
      canvas.width = 0;
      canvas.height = 0;
      page.cleanup();
    }
  });
  if (cancelled(signal)) return null;
  return canvas;
}

function previewTones(original: HTMLCanvasElement): HTMLCanvasElement | null {
  const { width, height } = original;
  const pixels = original.getContext("2d")?.getImageData(0, 0, width, height);
  if (pixels === undefined) return null;
  const tones = document.createElement("canvas");
  tones.width = width;
  tones.height = height;
  const context = tones.getContext("2d");
  if (context === null) return null;
  neutralTones(pixels, false);
  context.putImageData(pixels, 0, 0);
  return tones;
}

async function showTones(
  original: HTMLCanvasElement,
  scan: boolean,
  present: PagePaint["present"],
  full: boolean,
  signal: AbortSignal,
): Promise<void> {
  const tones = await mapTones(original, scan, signal).catch(() => null);
  if (signal.aborted) {
    tones?.close();
    original.width = 0;
    original.height = 0;
  } else {
    present(original, tones, full);
  }
}

async function mapTones(
  original: HTMLCanvasElement,
  scan: boolean,
  signal: AbortSignal,
): Promise<ImageBitmap | null> {
  if (
    signal.aborted ||
    typeof OffscreenCanvas === "undefined" ||
    typeof createImageBitmap === "undefined"
  )
    return null;
  const bitmap = await createImageBitmap(original);
  if (signal.aborted) {
    bitmap.close();
    return null;
  }
  const started = Result.try(
    () =>
      new Worker(new URL("./pdf_clear.worker.ts", import.meta.url), {
        type: "module",
      }),
  );
  if (Result.isFailure(started)) {
    bitmap.close();
    return null;
  }
  const worker = started.success;
  return new Promise<ImageBitmap | null>((resolve) => {
    const finish = (image: ImageBitmap | null): void => {
      worker.terminate();
      signal.removeEventListener("abort", cancel);
      resolve(image);
    };
    const cancel = (): void => finish(null);
    signal.addEventListener("abort", cancel, { once: true });
    worker.onmessage = (event: MessageEvent<ImageBitmap | null>) => finish(event.data);
    worker.onerror = cancel;
    const posted = Result.try(() => worker.postMessage({ bitmap, scan }, [bitmap]));
    if (Result.isFailure(posted)) cancel();
  }).finally(() => bitmap.close());
}

export function pagesText(pdf: PDFDocumentProxy): Promise<readonly string[]> {
  const numbers = Array.from({ length: pdf.numPages }, (_, index) => index + 1);
  return Promise.all(
    numbers.map((number) =>
      pdf
        .getPage(number)
        .then((page) => page.getTextContent())
        .then((content) =>
          content.items
            .map((item) => ("str" in item ? item.str + (item.hasEOL ? "\n" : "") : ""))
            .join(""),
        ),
    ),
  );
}
