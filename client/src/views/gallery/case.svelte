<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // One fixture, under the words naming the state it is in.
  //
  // Every fixture on this route is one of these, so the name is drawn
  // once and announced once: the label is the caption a person reads
  // and the region name `xtask render` reads, and two fixtures that
  // cannot be told apart by name are two unnamed regions in a screen
  // reader's jump list.
  //
  // wording-ok: the label is a state name written in English at the
  // call site, not a phrase from `lang.json`. This route is an
  // instrument for the people who build the client, and the name of a
  // state here is the name the source gives that state; a translated
  // caption would be a second spelling of it.
  //
  // It lives in a file of its own rather than in the index beside it
  // because every section imports it and the index imports every
  // section: putting it in the index would close that ring.
  //
  // **Every case states the width its subject is drawn at, and every
  // case draws the frame it is drawn in.** The route is as wide as the
  // window, so a row list stretched across 1920 pixels says nothing
  // about how it looks in the column it lives in; a 760-wide fixture
  // alone on that route, unframed, says instead that the page is
  // mostly empty, which is a worse lie. The dashed edge is the
  // difference: it says *this box is a specimen*, so the width belongs
  // to the subject and not to the page. A component takes the
  // conversation's width unless the case says otherwise; a page states
  // the window it is a page of; a container under test states its own
  // and is drawn at exactly that width.

  import type { Snippet } from "svelte";

  // The column a fixture is drawn in when its case states no width:
  // the conversation's, which is where a room's rows, notices,
  // banners, tables and composer all live.
  const TALK = 760;

  const FRAME =
    "min-w-0 rounded-card border border-dashed border-edge-input p-snug";

  interface CaseProps {
    readonly label: string;
    // The width its subject is drawn at, when that is the point of the
    // case. The subject is drawn at exactly this width; the frame's own
    // border and padding are not taken out of it.
    readonly width?: number;
    readonly children: Snippet;
  }
</script>

<script lang="ts">
  const { label, width, children }: CaseProps = $props();
</script>

<section class="mb-wide" aria-label={label}>
  <div class="mb-tight text-note text-text-faint">{label}</div>
  <!-- Two branches rather than one clever box: a stated width is a
  definite width, so the frame can shrink to it, while a default one
  has to come from the column the page already gives, and a `w-full`
  child inside a shrink-to-fit parent resolves to nothing at all. -->
  {#if width === undefined}
    <div class="w-full {FRAME}" style:max-width={TALK}>{@render children()}</div>
  {:else}
    <div class="w-max max-w-full {FRAME}">
      <div style:width="{String(width)}px" style:max-width="100%">{@render children()}</div>
    </div>
  {/if}
</section>
