<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The one thing in the client that draws an icon. Six files used to
// hand-draw their own `<svg>` for the same handful of shapes; every
// icon is now one name looked up in `glyph.ts` and drawn here, on one
// square with one stroke.
//
// The drawing is decorative without exception: a glyph names a control
// whose word is beside it, so the `<svg>` is always hidden from a
// screen reader and the caller puts the accessible name on the control.
//
// The stroke is 1.5 pixels wide with round caps and joins at either
// square, held to the screen rather than to the drawing's own units, so
// a mark inside a line of text is not drawn thinner than the one in a
// navigation column. Nothing here is coloured by hand: the stroke is
// `currentColor`, and the caller's text colour is the paint.

import type { ClassValue } from "svelte/elements";

import { GLYPHS, type GlyphName } from "./glyph";

// The two squares an icon may be drawn in (client-SPEC 4-34): sixteen
// for a mark that sits inside a line of text, twenty for the ones a
// column of navigation is built from.
export type GlyphSize = "sm" | "md";

export interface GlyphProps {
  readonly name: GlyphName;
  readonly size?: GlyphSize;
  readonly class?: ClassValue;
}

const BOX: Record<GlyphSize, string> = { sm: "size-glyph-sm", md: "size-glyph" };
</script>

<script lang="ts">
  const { name, size = "md", class: klass }: GlyphProps = $props();
</script>

<svg
  viewBox="0 0 20 20"
  class={[BOX[size], klass]}
  fill="none"
  stroke="currentColor"
  stroke-width="1.5"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  <path d={GLYPHS[name]} vector-effect="non-scaling-stroke" />
</svg>
