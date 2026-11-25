// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { GlobalWorkerOptions, PasswordException, getDocument } from "pdfjs-dist";
import type { PDFDocumentProxy } from "pdfjs-dist";
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
    const read = (cache: RequestCache) =>
      fetch(url, { cache }).then((response) =>
        response.ok
          ? response.arrayBuffer()
          : Promise.reject(new Error(`${filename}: ${String(response.status)}`)),
      );
    return read("default")
      .catch(() => read("reload"))
      .then((buffer) => new Uint8Array(buffer));
  }
}

const DOOR = "bundle/";

export type Opened =
  | { readonly kind: "open"; readonly pdf: PDFDocumentProxy; readonly detail: PdfDetail }
  | { readonly kind: "locked" }
  | { readonly kind: "broken"; readonly reason: string };

export interface Opening {
  readonly opened: Promise<Opened>;
  readonly close: () => void;
}

export type PdfDetail = () => Promise<PDFDocumentProxy>;

const PROGRESSIVE_FILE_BYTES = 4 * 1024 ** 2;
const PREVIEW_IMAGE_BYTES = 1024 ** 2;

function loadPdf(bytes: Uint8Array, imageBudget?: number) {
  return getDocument({
    data: bytes.slice(),
    BinaryDataFactory: Shipped,
    useWorkerFetch: false,
    cMapUrl: DOOR,
    cMapPacked: true,
    standardFontDataUrl: DOOR,
    wasmUrl: DOOR,
    enableXfa: false,
    enableHWA: webgl2Available(),
    ...(imageBudget === undefined ? {} : { canvasMaxAreaInBytes: imageBudget, stopAtErrors: true }),
  });
}

export function openPdf(bytes: Uint8Array, purpose: "reading" | "text" = "text"): Opening {
  const preview = purpose === "reading" && bytes.byteLength >= PROGRESSIVE_FILE_BYTES;
  const task = loadPdf(bytes, preview ? PREVIEW_IMAGE_BYTES : undefined);
  let sharp: Promise<PDFDocumentProxy> | undefined;
  let detailTask: ReturnType<typeof loadPdf> | undefined;
  let closed = false;
  function detail(): Promise<PDFDocumentProxy> {
    if (closed) return Promise.reject(new DOMException("PDF reader closed", "AbortError"));
    if (!preview) return task.promise;
    sharp ??= startDetail().catch((reason: unknown) =>
      closed ? Promise.reject(reason) : startDetail(),
    );
    return sharp;
  }
  function startDetail(): Promise<PDFDocumentProxy> {
    const load = loadPdf(bytes);
    detailTask = load;
    return load.promise.catch(async (reason: unknown) => {
      if (!closed && detailTask === load) {
        detailTask = undefined;
        await Promise.allSettled([load.destroy()]);
      }
      return Promise.reject(reason);
    });
  }
  return {
    opened: (preview ? task.promise.catch(recoverOpen) : task.promise).then(
      (pdf): Opened => ({ kind: "open", pdf, detail }),
      (reason: unknown): Opened =>
        reason instanceof PasswordException
          ? { kind: "locked" }
          : { kind: "broken", reason: String(reason) },
    ),
    close: () => {
      if (closed) return;
      closed = true;
      void Promise.allSettled(
        detailTask === undefined ? [task.destroy()] : [task.destroy(), detailTask.destroy()],
      );
    },
  };

  function recoverOpen(reason: unknown): Promise<PDFDocumentProxy> {
    return closed || reason instanceof PasswordException ? Promise.reject(reason) : detail();
  }
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
