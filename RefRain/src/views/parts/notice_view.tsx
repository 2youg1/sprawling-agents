// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { createMemo, For, Show } from "solid-js";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import Badge from "./badge.solid";
import { noticeOf, type Common, type NoticeLook, type Said, type Seat } from "./notice";

const PAINT: Readonly<Record<Seat, string>> = {
  toast:
    "rise rounded-panel border border-edge-panel bg-raised px-pane py-base shadow-float w-[min(var(--container-measure),calc(100vw-2*var(--spacing-pane)))] max-w-full",
  inline: "fade mt-tight rounded-card border border-edge-input px-snug py-tight",
  drawer: "fade w-full border-b border-edge px-base py-snug",
};

export type NoticeViewProps = Omit<Common, "actions"> &
  Said & {
    readonly actions?: (() => JSX.Element) | undefined;
  };

export default function Notice(props: NoticeViewProps) {
  const lang = storeValue(ui().lang);
  const fields = createMemo(() => noticeOf({ ...props, actions: undefined }, lang()));
  return <NoticeFrame look={fields()} actions={props.actions} />;
}

export interface NoticeFrameProps {
  readonly look: Omit<NoticeLook, "actions">;
  readonly actions?: (() => JSX.Element) | undefined;
}

export function NoticeFrame(props: NoticeFrameProps) {
  const look = () => props.look;
  return (
    <div
      role={look().region.role}
      class={[
        PAINT[look().seat],
        "flex min-w-0 flex-wrap items-start justify-between gap-base text-note",
      ]}
    >
      <div class="flex min-w-0 grow basis-[16rem] flex-col gap-tight">
        <div class="flex min-w-0 flex-wrap items-baseline gap-snug">
          <span
            class={[
              "min-w-0 font-label wrap-anywhere",
              look().weight === "alert" ? "text-alert" : "text-text",
            ]}
          >
            {look().title}
          </span>
          <Show when={look().at}>
            {(at) => <span class="shrink-0 text-text-faint">{at()}</span>}
          </Show>
          <Show when={look().count}>{(count) => <Badge text={count()} weight="quiet" />}</Show>
          <Show when={look().code}>
            {(code) => (
              <span class="ml-auto shrink-0 font-mono text-note text-text-faint">{code()}</span>
            )}
          </Show>
        </div>
        <Show when={look().next !== ""}>
          <p class="min-w-0 wrap-anywhere text-note text-text">{look().next}</p>
        </Show>
        <Show when={look().fold}>
          {(fold) => (
            <details class="min-w-0">
              <summary class="cursor-pointer text-note text-text-faint hover:text-text">
                {fold().summary}
              </summary>
              <div class="mt-tight flex min-w-0 flex-col gap-tight wrap-anywhere font-mono text-note text-text-quiet">
                <For each={fold().lines}>{(line) => <span>{line}</span>}</For>
              </div>
            </details>
          )}
        </Show>
      </div>
      <Show when={props.actions !== undefined}>
        <div class="ml-auto flex shrink-0 items-center gap-tight">{props.actions?.()}</div>
      </Show>
    </div>
  );
}
