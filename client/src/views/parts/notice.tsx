// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Something that happened, said once where a person is looking and kept
// where they can look again. The toast and the entry in the notification
// centre are one component in two seats, so a notice cannot be shown in
// one place and missing from the other.

import { Show, type JSX } from "solid-js";

// Something that happened, against something that was refused.
export type Weight = "info" | "alert";

// Floating over the page for a few seconds, or standing in the list that
// keeps it.
export type Seat = "toast" | "entry";

export interface NoticeProps {
  // Already in the person's language.
  readonly title: string;
  readonly detail?: string;
  // Already formatted by the caller's clock.
  readonly at?: string;
  readonly weight?: Weight;
  readonly seat?: Seat;
  // Usually a Button: undo, retry, or go to what this is about.
  readonly action?: JSX.Element;
  // Usually a quiet Button carrying the caller's word for dismissal.
  readonly dismiss?: JSX.Element;
}

export function Notice(props: NoticeProps) {
  const loud = () => props.weight === "alert";
  const floating = () => (props.seat ?? "toast") === "toast";
  return (
    <div
      role={loud() ? "alert" : "status"}
      class={`flex w-full min-w-0 items-start justify-between gap-base text-note transition-[opacity,transform] duration-200 ease-[cubic-bezier(0.2,0,0,1)] motion-reduce:transition-none ${
        floating()
          ? "max-w-measure rounded-panel border border-g3 bg-g1 px-pane py-base shadow-composer"
          : "border-b border-g2 px-base py-snug"
      }`}
    >
      <div class="flex min-w-0 flex-col gap-tight">
        <div class="flex min-w-0 items-baseline gap-snug">
          <span class={`truncate font-label ${loud() ? "text-alert" : "text-text"}`}>{props.title}</span>
          <Show when={props.at}>{(at) => <span class="shrink-0 text-text-faint">{at()}</span>}</Show>
        </div>
        <Show when={props.detail}>
          {(detail) => <span class="break-words text-text-quiet">{detail()}</span>}
        </Show>
      </div>
      <div class="flex shrink-0 items-center gap-tight">
        <Show when={props.action}>{(action) => action()}</Show>
        <Show when={props.dismiss}>{(dismiss) => dismiss()}</Show>
      </div>
    </div>
  );
}
