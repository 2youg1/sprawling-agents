// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For } from "solid-js";
import type { Lang } from "../../core/lang";
import { PdfLoading } from "../editor/formats/pdf_loading";
import { GalleryCase } from "./case";

interface Props {
  readonly bytes: Uint8Array;
  readonly version: string;
  readonly frame: string;
  readonly lang: Lang;
}

const CASES = [
  { label: "formats · a PDF, its pages drawn by pdf.js, Chinese through a CMap", night: false },
  { label: "formats · PDF with night mode on", night: true },
] as const;

export function PdfCase(props: Props) {
  return (
    <For each={CASES}>
      {(sample) => (
        <GalleryCase label={sample.label} width={600}>
          <div class={props.frame}>
            <PdfLoading
              bytes={props.bytes}
              version={props.version}
              name="brief.pdf"
              lang={props.lang}
              initialNight={sample.night}
            />
          </div>
        </GalleryCase>
      )}
    </For>
  );
}
