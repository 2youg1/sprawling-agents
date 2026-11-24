// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import Glyph from "../parts/glyph.solid";
import type { DisclosureLayout, DisclosureWire } from "./disclosure";

const COLUMNS: Record<DisclosureLayout, string> = {
  plan: "min-h-control grid-cols-[var(--spacing-glyph-sm)_minmax(0,1fr)_auto] gap-x-snug py-tight pr-snug",
  commit:
    "h-control grid-cols-[var(--spacing-glyph-sm)_8ch_minmax(0,1fr)_auto_auto] gap-x-base px-snug narrow:grid-cols-[var(--spacing-glyph-sm)_8ch_minmax(0,1fr)]",
  change:
    "min-h-control grid-cols-[var(--spacing-glyph-sm)_var(--spacing-figure)_minmax(0,1fr)_auto] gap-x-base py-tight px-snug",
};

interface Props {
  readonly layout: DisclosureLayout;
  readonly open: boolean;
  readonly wire: DisclosureWire;
  readonly children: JSX.Element;
}

export default function Disclosure(props: Props) {
  return (
    <button
      type={props.wire.type}
      aria-expanded={props.wire["aria-expanded"] ? "true" : "false"}
      data-control="surface"
      onClick={() => props.wire.onclick()}
      class={[
        "grid w-full items-center rounded-control text-left text-note",
        COLUMNS[props.layout],
      ]}
    >
      <Glyph
        name="chevron"
        size="sm"
        class={`text-text-faint transition-transform still:transition-none ${props.open ? "rotate-90 ease-arrive" : "ease-leave"}`}
      />
      {props.children}
    </button>
  );
}
