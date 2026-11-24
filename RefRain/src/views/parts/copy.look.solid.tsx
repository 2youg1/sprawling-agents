// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createUniqueId, Show } from "solid-js";
import Tip from "./control_tip";
import Glyph from "./glyph.solid";
import type { CopyForm, CopyLook } from "./copy";

const SHAPE: Record<CopyForm, string> = {
  named: "h-control gap-tight px-snug text-label",
  bare: "size-control-sm justify-center",
};

export default function CopyLookView(look: CopyLook) {
  const hint = createUniqueId();
  return (
    <>
      <Tip text={look.note} id={hint}>
        <button
          type={look.key.type}
          data-control=""
          data-control-state={look.refused ? "refused" : look.said ? "success" : undefined}
          aria-label={look.key["aria-label"]}
          aria-describedby={hint}
          onClick={() => look.key.onclick()}
          class={[
            "relative flex shrink-0 items-center rounded-control before:absolute before:-inset-snug before:content-['']",
            "transition-[background-color,color] ease-leave hover:bg-raised hover:ease-arrive still:transition-none",
            SHAPE[look.form],
            look.refused ? "text-alert" : "text-text-quiet hover:text-text",
          ]}
        >
          <Glyph name={look.face} size="sm" class="shrink-0" />
          <Show when={look.form === "named"}>{look.name}</Show>
        </button>
      </Tip>
      <span class="sr-only" {...look.heard}>
        {look.said}
      </span>
    </>
  );
}
