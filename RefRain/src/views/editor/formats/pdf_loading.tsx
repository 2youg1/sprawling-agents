// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { lazy, Loading } from "solid-js";
import { createEffect, createMemo, createSignal, Match, Switch } from "solid-js";
import { fill, say, type Lang } from "../../../core/lang";
import { NAME } from "./format";
import type { Opened } from "./pdf";
import type { PdfProgress, PdfReference, PdfSelection } from "./pdf_anchor";

const PdfReading = lazy(() =>
  import("./pdf_view").then(({ PdfReading }) => ({ default: PdfReading })),
);

export function preloadPdfReader(): void {
  void PdfReading.preload().catch(() => undefined);
}

export interface PdfProps {
  readonly bytes: Uint8Array;
  readonly version: string | null;
  readonly name: string;
  readonly lang: Lang;
  readonly progress?: PdfProgress | null | undefined;
  readonly onProgress?: ((progress: PdfProgress) => void) | undefined;
  readonly reference?: PdfReference | null | undefined;
  readonly onSelection?: ((attachment: PdfSelection) => void) | undefined;
}

export interface Graphics {
  readonly webgpu: boolean;
  readonly webgl2: boolean;
}

function PdfView(props: PdfProps) {
  const [opened, setOpened] = createSignal<Opened | null>(null);
  const [graphics, setGraphics] = createSignal<Graphics>();
  const bytes = createMemo(() => props.bytes);
  const pdf = createMemo(() => {
    const value = opened();
    return value?.kind === "open" ? value : null;
  });
  const broken = createMemo(() => {
    const value = opened();
    return value?.kind === "broken" ? value.reason : undefined;
  });

  createEffect(bytes, (from) => {
    setOpened(null);
    let gone = false;
    let close: (() => void) | undefined;
    preloadPdfReader();
    void import("./pdf")
      .then(({ openPdf, gpuAvailability }) => {
        if (gone) return;
        const opening = openPdf(from, "reading");
        close = opening.close;
        void opening.opened.then((result) => {
          if (!gone) setOpened(result);
        });
        void gpuAvailability().then((available) => {
          if (!gone) setGraphics(available);
        });
      })
      .catch((reason: unknown) => {
        if (!gone) setOpened({ kind: "broken", reason: String(reason) });
      });
    return () => {
      gone = true;
      close?.();
    };
  });

  return (
    <Switch>
      <Match when={pdf()} keyed>
        {(document) => (
          <PdfReading
            {...props}
            pdf={document.pdf}
            detailDocument={document.detail}
            graphics={graphics()}
          />
        )}
      </Match>
      <Match when={opened()?.kind === "locked"}>
        <p class="px-wide py-snug text-note text-text-quiet">{say(props.lang, "format_locked")}</p>
      </Match>
      <Match when={broken()}>
        {(reason) => (
          <p class="px-wide py-snug text-note text-text-quiet">
            {fill(say(props.lang, "format_broken"), { format: NAME.pdf, reason: reason() })}
          </p>
        )}
      </Match>
      <Match when={opened() === null}>
        <p class="px-wide py-snug text-note text-text-faint">
          {fill(say(props.lang, "format_reading"), { format: NAME.pdf })}
        </p>
      </Match>
    </Switch>
  );
}

export function PdfLoading(props: PdfProps) {
  return (
    <Loading fallback={null}>
      <PdfView {...props} />
    </Loading>
  );
}
