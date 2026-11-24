// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { PDFDocumentProxy } from "pdfjs-dist";
import { createEffect, createSignal, onCleanup, Show } from "solid-js";

import { fill, say } from "../../../core/lang";
import type { Lang } from "../../../core/lang";
import type { Size } from "./pdf";

interface PageProps {
  readonly pdf: PDFDocumentProxy;
  readonly page: number;
  readonly width: number;
  readonly lang: Lang;
  readonly measured?: (size: Size) => void;
  readonly selectable?: boolean;
}

function clear(surface: HTMLElement): void {
  for (const canvas of surface.querySelectorAll("canvas")) {
    canvas.width = 0;
    canvas.height = 0;
  }
  surface.replaceChildren();
}

function display(surface: HTMLElement, picture: HTMLCanvasElement): void {
  clear(surface);
  picture.className = "pdf-original";
  surface.append(picture);
}

function enhance(surface: HTMLElement, bitmap: ImageBitmap): void {
  const canvas = document.createElement("canvas");
  canvas.width = bitmap.width;
  canvas.height = bitmap.height;
  canvas.className = "pdf-clear";
  const context = canvas.getContext("bitmaprenderer");
  if (context === null) canvas.getContext("2d")?.drawImage(bitmap, 0, 0);
  else context.transferFromImageBitmap(bitmap);
  bitmap.close();
  surface.append(canvas);
}

export function PdfPage(props: PageProps) {
  let pixels: HTMLDivElement | undefined;
  let words: HTMLDivElement | undefined;
  const [failed, setFailed] = createSignal<string>();
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
      let stop: (() => void) | undefined;
      let gone = false;
      setFailed(undefined);
      const failure = (reason: unknown): void => {
        if (!gone) setFailed(String(reason));
      };
      void import("./pdf")
        .then(({ drawPdf }) => {
          if (gone) return;
          stop = drawPdf({
            pdf,
            number,
            width,
            present: (picture, tones) => {
              display(surface, picture);
              if (tones !== null) enhance(surface, tones);
            },
            measure: (size) => {
              props.measured?.(size);
            },
            failure,
            ...(words === undefined || selectable === false ? {} : { textLayer: words }),
          });
        })
        .catch(failure);
      return () => {
        gone = true;
        stop?.();
      };
    },
  );
  onCleanup(() => {
    if (pixels !== undefined) clear(pixels);
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
