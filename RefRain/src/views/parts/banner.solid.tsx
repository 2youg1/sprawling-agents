// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { Show, createMemo } from "solid-js";

import { bannerOf, type BannerProps, type Weight } from "./banner";

const PAINT: Record<Weight, string> = {
  notice: "border-edge-panel text-text-quiet",
  alert: "border-alert/50 text-alert",
};

export default function Banner(props: BannerProps<JSX.Element>) {
  const look = createMemo(() => bannerOf(props));
  return (
    <div
      {...look().strip}
      class={[
        "drop flex w-full min-w-0 items-center justify-between gap-base border-b bg-raised px-pane py-snug text-note",
        PAINT[look().weight],
      ]}
    >
      <div class="flex min-w-0 flex-wrap items-baseline gap-x-snug wrap-anywhere">
        <span>{look().text}</span>
        <Show when={look().detail !== undefined}>
          <span class="text-text-faint">{look().detail}</span>
        </Show>
      </div>
      <Show when={look().action !== undefined}>
        <div class="shrink-0">{look().action}</div>
      </Show>
    </div>
  );
}
