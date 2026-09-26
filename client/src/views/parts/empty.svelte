<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What stands where a list has nothing in it: a shape the eye lands on,
// one sentence saying what is missing, and the one action that ends the
// emptiness. A grey word on its own leaves a person unsure whether the
// page is empty or broken.
//
// The shape is decoration and is hidden from a screen reader; the
// sentence and the action are the whole readable content. The sentence
// arrives as a key rather than as a word, so what a person is told here
// has exactly one home, `lang.json`, like every other word on a screen.

import type { Snippet } from "svelte";

import type { Key } from "../../core/lang";

export interface EmptyStateProps {
  // The key of the sentence saying what this screen is missing.
  readonly missing: Key;
  // The outline the eye lands on, when the page has one worth drawing.
  // Without it the sentence stands alone: an empty outline reads as a
  // box that failed to load.
  readonly shape?: Snippet;
  // Usually one Button: the way out of the emptiness.
  readonly action?: Snippet;
  // Where the sentence sits. `centred` stands alone in a narrow column;
  // `region` stands in for a list or a table on a wide page, so it is
  // drawn where that region would be - its left edge, its width, and a
  // dashed outline of it - rather than as a line floating in the middle
  // of the window. `inset` is the same seat inside a container that
  // already draws the region's outline, such as an empty table's frame.
  readonly seat?: EmptySeat;
}

export type EmptySeat = "centred" | "region" | "inset";

const SEATS: Record<EmptySeat, string> = {
  centred: "items-center px-pane py-section text-center",
  region: "items-start rounded-card border border-dashed border-edge-input px-wide py-wide text-left",
  inset: "items-start px-wide py-wide text-left",
};
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { ui } from "../../ui";

  const { missing, shape, action, seat = "centred" }: EmptyStateProps = $props();

  const { lang } = ui();
</script>

<div class={["flex w-full flex-col gap-base", SEATS[seat]]}>
  {#if shape !== undefined}
    <div aria-hidden="true">
      {@render shape()}
    </div>
  {/if}
  <p class="max-w-measure text-note text-text-quiet">{say($lang, missing)}</p>
  {#if action !== undefined}
    {@render action()}
  {/if}
</div>
