// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { dynamic, type JSX } from "@solidjs/web";
import { Show } from "solid-js";
import type { PageProps } from "./page";

export default function Page(props: PageProps<JSX.Element>) {
  const isPage = () => (props.rank ?? "page") === "page";
  const Heading = dynamic(() => (isPage() ? "h1" : "h2"));
  return (
    <div class="flex w-full min-w-0 flex-1 flex-col gap-wide pb-section">
      <header class="flex min-w-0 flex-wrap items-end justify-between gap-x-wide gap-y-snug border-b border-edge pb-base">
        <div class="flex min-w-0 flex-col gap-tight">
          <Heading class="text-title font-title" tabindex={isPage() ? -1 : undefined}>
            {props.title}
          </Heading>
          <Show when={props.note}>
            {(note) => <p class="text-note text-text-quiet">{note()}</p>}
          </Show>
        </div>
        <Show when={props.aside}>
          {(aside) => <div class="flex min-w-0 flex-wrap items-center gap-base">{aside()}</div>}
        </Show>
      </header>
      {props.children}
    </div>
  );
}
