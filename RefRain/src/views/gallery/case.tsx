// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { Show } from "solid-js";

import { FRAME, TALK } from "./case_frame";

interface CaseProps {
  readonly label: string;
  readonly width?: number;
  readonly children: JSX.Element;
}

export function GalleryCase(props: CaseProps) {
  return (
    <section class="mb-wide" aria-label={props.label}>
      <div class="mb-tight text-note text-text-faint">{props.label}</div>
      <Show
        when={props.width !== undefined}
        fallback={
          <div class={`w-full ${FRAME}`} style={{ "max-width": `${String(TALK)}px` }}>
            {props.children}
          </div>
        }
      >
        <div class={`w-max max-w-full ${FRAME}`}>
          <div
            class="@container"
            style={{ width: `${String(props.width)}px`, "max-width": "100%" }}
          >
            {props.children}
          </div>
        </div>
      </Show>
    </section>
  );
}
