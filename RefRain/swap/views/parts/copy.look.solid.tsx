// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createUniqueId, Show } from "solid-js";
import type { CopyLook } from "../../../src/views/parts/copy";
import Tip from "../../../src/views/parts/control_tip";
import Glyph from "../../../src/views/parts/glyph.solid";
import "./copy.solid.css";

export default function AlternateCopy(look: CopyLook) {
  const note = createUniqueId();
  function Key() {
    return (
      <button
        class="swap-copy-key"
        type={look.key.type}
        data-control=""
        data-control-state={look.refused ? "refused" : look.said ? "success" : undefined}
        aria-label={look.key["aria-label"]}
        aria-describedby={note}
        onClick={() => look.key.onclick()}
      >
        <Glyph name={look.face} size="sm" />
        <Show when={look.form === "named"}>{look.name}</Show>
      </button>
    );
  }
  return (
    <span class="swap-copy">
      <Show
        when={look.form === "bare"}
        fallback={
          <>
            <Key />
            <span class="swap-copy-note" id={note}>
              {look.note}
            </span>
          </>
        }
      >
        <Tip text={look.note} id={note}>
          <Key />
        </Tip>
      </Show>
      <span class="sr-only" {...look.heard}>
        {look.said}
      </span>
    </span>
  );
}
