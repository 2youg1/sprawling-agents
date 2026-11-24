// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import {
  GlobalWorkerOptions,
  OPS,
  PasswordException,
  RenderingCancelledException,
  TextLayer,
  getDocument,
} from "pdfjs-dist";
import { Result } from "effect";
import type { PDFDocumentProxy, PDFPageProxy } from "pdfjs-dist";
import worker from "pdfjs-dist/build/pdf.worker.min.mjs?url";

GlobalWorkerOptions.workerSrc = worker;

const SHIPPED: ReadonlyMap<string, string> = new Map(
  Object.entries(
    import.meta.glob<string>(
      [
        "../../../../node_modules/pdfjs-dist/cmaps/*.bcmap",
        "../../../../node_modules/pdfjs-dist/standard_fonts/Foxit{Symbol,Dingbats}.pfb",
        "../../../../node_modules/pdfjs-dist/wasm/{openjpeg,jbig2,qcms_bg}.wasm",
      ],
      { query: "?url", import: "default", eager: true },
    ),
  ).map(([path, url]) => [path.slice(path.lastIndexOf("/") + 1), url]),
);

class Shipped {
  fetch({ filename }: { readonly kind: string; readonly filename: string }): Promise<Uint8Array> {
    const url = SHIPPED.get(filename);
    if (url === undefined)
      return Promise.reject(new Error(`${filename} is not part of this bundle`));
    return fetch(url)
      .then((response) =>
        response.ok
          ? response.arrayBuffer()
          : Promise.reject(new Error(`${filename}: ${String(response.status)}`)),
      )
      .then((buffer) => new Uint8Array(buffer));
  }
}

const DOOR = "bundle/";

export type Opened =
  | { readonly kind: "open"; readonly pdf: PDFDocumentProxy }
  | { readonly kind: "locked" }
  | { readonly kind: "broken"; readonly reason: string };

export interface Opening {
  readonly opened: Promise<Opened>;
  readonly close: () => void;
}

export function openPdf(bytes: Uint8Array): Opening {
  const task = getDocument({
    data: bytes.slice(),
    BinaryDataFactory: Shipped,
    useWorkerFetch: false,
    cMapUrl: DOOR,
    cMapPacked: true,
    standardFontDataUrl: DOOR,
    wasmUrl: DOOR,
    enableXfa: false,
    enableHWA: webgl2Available(),
  });
  return {
    opened: task.promise.then(
      (pdf): Opened => ({ kind: "open", pdf }),
      (reason: unknown): Opened =>
        reason instanceof PasswordException
          ? { kind: "locked" }
          : { kind: "broken", reason: String(reason) },
    ),
    close: () => {
      void task.destroy();
    },
  };
}

export interface Size {
  readonly width: number;
  readonly height: number;
}

const MAX_PIXELS = 4_194_304;
const MAX_SIDE = 4096;

export interface PagePaint {
  readonly pdf: PDFDocumentProxy;
  readonly number: number;
  readonly width: number;
  readonly present: (original: HTMLCanvasElement, tones: ImageBitmap | null) => void;
  readonly measure: (size: Size) => void;
  readonly failure: (reason: unknown) => void;
  readonly textLayer?: HTMLDivElement;
}

function raster(page: PDFPageProxy, width: number, density: number) {
  const natural = page.getViewport({ scale: 1 });
  const scale = Math.min(
    (width * density) / natural.width,
    Math.sqrt(MAX_PIXELS / (natural.width * natural.height)),
    MAX_SIDE / Math.max(natural.width, natural.height),
  );
  return page.getViewport({ scale });
}

export function drawPdf(options: PagePaint): () => void {
  const abort = new AbortController();
  void paint(options, abort.signal).catch((reason: unknown) => {
    if (!abort.signal.aborted && !(reason instanceof RenderingCancelledException)) {
      options.failure(reason);
    }
  });
  return () => {
    abort.abort();
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

async function paint(options: PagePaint, signal: AbortSignal): Promise<void> {
  const page = await options.pdf.getPage(options.number);
  if (cancelled(signal)) return;
  signal.addEventListener(
    "abort",
    () => {
      page.cleanup();
    },
    { once: true },
  );
  const natural = page.getViewport({ scale: 1 });
  options.measure({ width: natural.width, height: natural.height });
  const words = options.textLayer;
  if (words !== undefined) paintText(page, options.width, words, signal, options.failure);

  const preview = raster(page, options.width, 0.8);
  const display = raster(page, options.width, Math.max(1.35, window.devicePixelRatio));
  const first = await renderRaster(page, preview, signal);
  if (first === null || cancelled(signal)) return;
  await showTones(first, false, options.present, signal);
  if (cancelled(signal)) return;
  if (Math.floor(display.width) === Math.floor(preview.width)) return;
  const original = await renderRaster(page, display, signal);
  if (original === null || cancelled(signal)) return;
  const operators = await page.getOperatorList().catch(() => null);
  await showTones(original, operators !== null && scanned(operators), options.present, signal);
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
): Promise<HTMLCanvasElement | null> {
  if (signal.aborted) return null;
  const canvas = document.createElement("canvas");
  canvas.width = Math.max(1, Math.floor(viewport.width));
  canvas.height = Math.max(1, Math.floor(viewport.height));
  const task = page.render({ canvas, viewport });
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

async function showTones(
  original: HTMLCanvasElement,
  scan: boolean,
  present: PagePaint["present"],
  signal: AbortSignal,
): Promise<void> {
  const tones = await mapTones(original, scan, signal).catch(() => null);
  if (signal.aborted) {
    tones?.close();
    original.width = 0;
    original.height = 0;
  } else {
    present(original, tones);
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

let capturedWebgl: boolean | undefined;

function webgl2Available(): boolean {
  if (capturedWebgl !== undefined) return capturedWebgl;
  const probe = document.createElement("canvas").getContext("webgl2");
  capturedWebgl = probe !== null;
  probe?.getExtension("WEBGL_lose_context")?.loseContext();
  return capturedWebgl;
}

export function gpuAvailability(): Promise<{ webgpu: boolean; webgl2: boolean }> {
  const webgl2 = webgl2Available();
  if (!Reflect.has(navigator, "gpu")) return Promise.resolve({ webgpu: false, webgl2 });
  return navigator.gpu.requestAdapter().then(
    (adapter) => ({ webgpu: adapter !== null, webgl2 }),
    () => ({ webgpu: false, webgl2 }),
  );
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
