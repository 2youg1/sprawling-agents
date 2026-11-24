// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createUniqueId, Show } from "solid-js";
import type { ButtonLook, ButtonResult, Tone } from "./button";
import Tip from "./control_tip";
import Glyph from "./glyph.solid";
import type { GlyphName } from "./glyph";

const WEAR: Record<Tone, string> = {
  primary: "h-control-lg bg-accent text-on-accent data-[state=idle]:hover:bg-accent-hover",
  secondary: "h-control bg-raised text-text data-[state=idle]:hover:bg-raised-hover",
  quiet: "h-control text-text-quiet data-[state=idle]:hover:bg-raised",
  destructive:
    "h-control bg-raised text-alert data-[state=idle]:hover:bg-alert data-[state=idle]:hover:text-on-accent",
};

const MUTED = "aria-disabled:bg-raised aria-disabled:text-text-disabled";

const SHAPE = "inline-flex items-center gap-snug rounded-control px-base text-label";

const RESULTS: Record<ButtonResult, GlyphName> = {
  success: "check",
  refused: "cross",
  unknown: "search",
};

export default function Look(look: ButtonLook) {
  const id = createUniqueId();
  const wire = () => look.wire(look.why === undefined ? undefined : id);
  const glyph = () => (look.result === undefined ? look.glyph : RESULTS[look.result]);
  return (
    <Tip id={id} text={look.why}>
      <button
        type={wire().type}
        aria-disabled={wire()["aria-disabled"] ? "true" : "false"}
        aria-busy={wire()["aria-busy"] ? "true" : "false"}
        aria-label={wire()["aria-label"]}
        aria-describedby={wire()["aria-describedby"]}
        data-control=""
        data-control-state={look.state === "loading" ? "pending" : look.result}
        data-state={look.state}
        class={[SHAPE, WEAR[look.tone], MUTED]}
        onClick={(event) => {
          if (wire()["aria-disabled"]) event.preventDefault();
          wire().onclick();
        }}
      >
        <Show
          when={look.state !== "loading"}
          fallback={
            <span class="inline-block size-dot pulse rounded-pill bg-current" aria-hidden="true" />
          }
        >
          <Show when={glyph()} keyed>
            {(name) => <Glyph name={name} size="sm" class="shrink-0" />}
          </Show>
        </Show>
        {look.label}
      </button>
    </Tip>
  );
}
