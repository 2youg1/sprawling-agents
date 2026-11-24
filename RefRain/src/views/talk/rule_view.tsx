// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Show } from "solid-js";
import Glyph from "../parts/glyph.solid";
import type { FoldLook } from "./fold";
import type { RuleLook, RulePart } from "./rule";

export function AsideLink(props: { readonly href: string; readonly text: string }) {
  return (
    <a
      href={props.href}
      class="underline decoration-from-font underline-offset-2 transition-colors ease-leave hover:text-text hover:ease-arrive"
    >
      {props.text}
    </a>
  );
}

export function Fold(props: FoldLook) {
  return (
    <button
      type={props.wire.type}
      data-control="surface"
      aria-expanded={props.wire["aria-expanded"] ? "true" : "false"}
      onClick={() => props.wire.onclick()}
      class="rounded-control px-tight hover:text-text-quiet"
    >
      {props.label}
    </button>
  );
}

function draw(part: RulePart) {
  switch (part.kind) {
    case "text":
      return <span>{part.text}</span>;
    case "link":
      return <AsideLink text={part.text} href={part.href} />;
    case "fold":
      return <Fold {...part.fold} />;
  }
}

export default function Rule(props: RuleLook) {
  return (
    <div
      role={props.wire.role}
      data-wear={props.wire["data-wear"]}
      class="flex items-center gap-base text-note text-text-faint"
    >
      <span class="h-px flex-1 bg-raised" />
      <span class="flex min-w-0 flex-wrap items-center justify-center gap-x-tight">
        <Show when={props.mark === "branch"}>
          <Glyph name="branch" size="sm" />
        </Show>
        <For each={props.parts} keyed={false}>
          {(part, at) => (
            <>
              <Show when={at > 0}>
                <span aria-hidden="true">·</span>
              </Show>
              {draw(part())}
            </>
          )}
        </For>
      </span>
      <span class="h-px flex-1 bg-raised" />
    </div>
  );
}
