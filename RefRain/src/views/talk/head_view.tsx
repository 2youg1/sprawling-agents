// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { createMemo, For, Show } from "solid-js";
import { fill, say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { hhmmss, isoInstant } from "../../core/time";
import { ui } from "../../ui";
import { SLICES } from "./rhythm";
import { tookWords, type Took } from "./timing";

const HEIGHT = 12;

export function Sparkline(props: { readonly rhythm: readonly number[] }) {
  return (
    <svg
      class="inline-block h-sparkline-tall w-sparkline shrink-0 self-center text-text-faint"
      viewBox={`0 0 ${String(SLICES * 2)} ${String(HEIGHT)}`}
      aria-hidden="true"
    >
      <For each={props.rhythm} keyed={false}>
        {(share, slice) => {
          const tall = createMemo(() => Math.max(1, Math.round(share() * HEIGHT)));
          return (
            <Show when={share() > 0}>
              <rect
                x={slice * 2}
                y={HEIGHT - tall()}
                width="1"
                height={tall()}
                fill="currentColor"
              />
            </Show>
          );
        }}
      </For>
    </svg>
  );
}

export function HeadLookView(props: {
  readonly who: string;
  readonly model: string | null;
  readonly time: { readonly text: string; readonly iso: string };
  readonly ttft?: string | undefined;
  readonly tps?: string | undefined;
  readonly rhythm?: JSX.Element;
}) {
  return (
    <div class="min-w-0 text-note text-text-faint">
      <span class="me-tight font-label text-label whitespace-nowrap text-text">{props.who}</span>
      <Show when={props.model}>
        {(model) => <span class="me-tight wrap-anywhere">{model()}</span>}
      </Show>
      <time class="figure me-tight whitespace-nowrap" datetime={props.time.iso}>
        {props.time.text}
      </time>
      <Show when={props.ttft}>
        {(ttft) => <span class="figure me-tight whitespace-nowrap">{ttft()}</span>}
      </Show>
      <Show when={props.tps}>
        {(tps) => <span class="figure me-tight whitespace-nowrap">{tps()}</span>}
      </Show>
      {props.rhythm}
    </div>
  );
}

export default function Head(props: {
  readonly who: string;
  readonly at: number;
  readonly model: string | null;
  readonly ttft: Took | null;
  readonly tps: number | null;
  readonly rhythm: readonly number[] | null;
}) {
  const lang = storeValue(ui().lang);
  return (
    <HeadLookView
      who={props.who}
      model={props.model}
      time={{ text: hhmmss(props.at), iso: isoInstant(props.at) }}
      ttft={
        props.ttft === null
          ? undefined
          : `${say(lang(), "talk_ttft")} ${tookWords(props.ttft, lang())}`
      }
      tps={
        props.tps === null
          ? undefined
          : fill(say(lang(), "talk_tps"), { n: String(Math.round(props.tps)) })
      }
      rhythm={<Show when={props.rhythm}>{(points) => <Sparkline rhythm={points()} />}</Show>}
    />
  );
}
