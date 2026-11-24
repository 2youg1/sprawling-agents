// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { Show } from "solid-js";

import Reach, { type ReachProps } from "./reach.solid";

export interface CrumbProps extends ReachProps {
  readonly children?: JSX.Element;
}

export default function Crumb(props: CrumbProps) {
  const cut = () => props.path.lastIndexOf("/") + 1;
  return (
    <div class="flex h-[calc(4*var(--spacing-baseline))] shrink-0 items-center gap-base border-b border-edge pr-snug pl-wide text-note text-text-faint">
      <span class="min-w-0 flex-1 truncate font-mono">
        {props.path.slice(0, cut())}
        <span class="font-label text-text-quiet">{props.path.slice(cut())}</span>
      </span>
      <Show when={props.children !== undefined}>
        <span class="shrink-0 whitespace-nowrap">{props.children}</span>
      </Show>
      <Reach path={props.path} line={props.line} shown={props.shown} />
    </div>
  );
}
