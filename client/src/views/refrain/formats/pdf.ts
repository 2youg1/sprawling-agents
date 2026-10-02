// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// pdf.js, set up once for this client (client-SPEC 4-54, 12-32). This
// module is its own lazy chunk: nothing of pdf.js is fetched until a
// PDF is drawn. The worker parses in its own thread; the resources it
// asks for by name - the Adobe CMaps a font without its own encoding
// table needs, the two symbol faces no machine is sure to have, and the
// image decoders built as WebAssembly - are files of this bundle, found
// by name in `SHIPPED` and handed over by the page, so nothing is
// fetched from anywhere but the city that served the page. Every other
// face a file leaves out is drawn with the machine's own, as Firefox's
// own viewer does.

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

// The page's side of pdf.js's resource door: the worker names a file,
// and this answers with that file of the bundle, or refuses a name the
// bundle does not carry.
class Shipped {
  fetch({ filename }: { readonly kind: string; readonly filename: string }): Promise<Uint8Array> {
    const url = SHIPPED.get(filename);
    if (url === undefined) return Promise.reject(new Error(`${filename} is not part of this bundle`));
    return fetch(url)
      .then((response) => (response.ok ? response.arrayBuffer() : Promise.reject(new Error(`${filename}: ${String(response.status)}`))))
      .then((buffer) => new Uint8Array(buffer));
  }
}

// pdf.js wants a base for each resource kind, and asks the door above
// for a name under it; the base is never fetched.
const DOOR = "bundle/";

export type Opened =
  | { readonly kind: "open"; readonly pdf: PDFDocumentProxy }
  | { readonly kind: "locked" }
  | { readonly kind: "broken"; readonly reason: string };

export interface Opening {
  readonly opened: Promise<Opened>;
  readonly close: () => void;
}

// Opens a PDF from its bytes. The bytes are copied, because pdf.js
// hands its buffer to the worker and leaves the caller's empty.
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
  });
  return {
    opened: task.promise.then(
      (pdf): Opened => ({ kind: "open", pdf }),
      (reason: unknown): Opened =>
        reason instanceof PasswordException ? { kind: "locked" } : { kind: "broken", reason: String(reason) },
    ),
    close: () => {
      void task.destroy();
    },
  };
}

// A page's size in PDF points, which fixes its proportions.
export interface Size {
  readonly width: number;
  readonly height: number;
}

export interface Drawing {
  readonly pdf: PDFDocumentProxy;
  readonly number: number;
  readonly canvas: HTMLCanvasElement;
  // The width the page takes on the screen, in CSS pixels.
  readonly width: number;
}

// One page drawn into its canvas, sharp on this screen; `measured`
// hears the page's own size before it is drawn. Answers the way to stop
// a drawing the page no longer needs.
export function drawPage(drawing: Drawing, measured: (size: Size) => void): () => void {
  const { pdf, number, canvas, width } = drawing;
  let cancel = (): void => undefined;
  let gone = false;
  void pdf.getPage(number).then((page) => {
    if (gone) return;
    const natural = page.getViewport({ scale: 1 });
    measured({ width: natural.width, height: natural.height });
    const viewport = page.getViewport({ scale: (width / natural.width) * window.devicePixelRatio });
    canvas.width = Math.floor(viewport.width);
    canvas.height = Math.floor(viewport.height);
    const task = page.render({ canvas, viewport });
    cancel = () => {
      task.cancel();
    };
    task.promise.catch(() => undefined);
  });
  return () => {
    gone = true;
    cancel();
  };
}

// The text of each page in reading order, a line where the file ends
// one. A page whose text is empty is drawn but has no words to compare.
export function pagesText(pdf: PDFDocumentProxy): Promise<readonly string[]> {
  const numbers = Array.from({ length: pdf.numPages }, (_, index) => index + 1);
  return Promise.all(
    numbers.map((number) =>
      pdf
        .getPage(number)
        .then((page) => page.getTextContent())
        .then((content) =>
          content.items.map((item) => ("str" in item ? item.str + (item.hasEOL ? "\n" : "") : "")).join(""),
        ),
    ),
  );
}
