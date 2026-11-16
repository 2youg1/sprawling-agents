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

export interface Drawing {
  readonly pdf: PDFDocumentProxy;
  readonly number: number;
  readonly canvas: HTMLCanvasElement;
  
  readonly width: number;
}


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
