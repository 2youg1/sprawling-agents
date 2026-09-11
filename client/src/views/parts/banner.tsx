// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A strip across the top of a page for the condition the whole page is
// under: the city is stopped, the link is down, somebody is waiting to
// be answered. It is not a field's error and not a toast - it belongs to
// the page, it stays while the condition lasts, and it carries the one
// action that ends the condition.

import { Show, type JSX } from "solid-js";

// A condition a person should know about, against one they must act on.
export type Weight = "notice" | "alert";

export interface BannerProps {
  // Already in the person's language.
  readonly text: string;
  readonly detail?: string;
  readonly weight?: Weight;
  // Usually a Button: the way out of the condition.
  readonly action?: JSX.Element;
}

export function Banner(props: BannerProps) {
  const loud = () => props.weight === "alert";
  return (
    <div
      role={loud() ? "alert" : "status"}
      class={`flex w-full min-w-0 items-center justify-between gap-base border-b px-pane py-snug text-note transition-[opacity,transform] duration-200 ease-[cubic-bezier(0.2,0,0,1)] motion-reduce:transition-none ${
        loud() ? "border-alert/50 bg-g1 text-alert" : "border-g3 bg-g1 text-text-quiet"
      }`}
    >
      <div class="flex min-w-0 items-baseline gap-snug">
        <span class="truncate">{props.text}</span>
        <Show when={props.detail}>
          {(detail) => <span class="truncate text-text-faint">{detail()}</span>}
        </Show>
      </div>
      <Show when={props.action}>{(action) => <div class="shrink-0">{action()}</div>}</Show>
    </div>
  );
}
