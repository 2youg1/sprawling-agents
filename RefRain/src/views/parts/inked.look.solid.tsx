// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For } from "solid-js";
import type { Ink, InkedLook } from "./code";
const PAINT: Record<Ink, string> = {
  plain: "",
  comment: "text-text-faint",
  string: "text-alert",
  number: "text-accent",
  word: "text-text",
};

export type PortProps = InkedLook;

export default function PartsInkedLook(look: InkedLook) {
  return (
    <>
      <For
        each={Array.from(look.pieces ?? []).map((__rrItem0, __rrIndex1) => ({
          value: __rrItem0,
          key: __rrItem0,
        }))}
        keyed={(entry) => entry.key}
      >
        {(__rrItem0) => (
          <>
            <span class={PAINT[__rrItem0().value.ink]}>{__rrItem0().value.text}</span>
          </>
        )}
      </For>
    </>
  );
}
