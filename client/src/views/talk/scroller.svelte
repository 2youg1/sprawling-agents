<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- The column the conversation scrolls in, and the one place that
decides where its view stands while words arrive (refrain §3-14, the
first row; client/Spec.lean §4-44).

**The view follows only a person standing at the foot.** Scrolling up to
read, or holding a selection inside the thread, keeps the place; what
arrives meanwhile is counted, and the count is a button that goes back to
the foot. Back at the foot, by that button or by scrolling, the view
follows again. The judgement is `anchoring.ts`'s; this file owns the
scroller and is the only reader of its geometry besides the read-wear
bar.

**What is counted is a block the thread names** - each round, each
ending line - by the same `data-wear` attribute the bar reads, so "3
new" means three things a person would read, not three resize events.

A font arriving, a code block taking its colours or a reply closing its
Markdown changes the height of text already there; in the held state the
engine's own scroll anchoring keeps the paragraph under the eye, and in
the following state the view is at the foot either way. -->
<script lang="ts">
  import { untrack } from "svelte";
  import type { Snippet } from "svelte";

  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import Glyph from "../parts/glyph.svelte";
  import { anchorAt, footOf } from "./anchoring";
  import type { Anchoring } from "./anchoring";
  import Wear from "./wear.svelte";

  interface Props {
    // The room has nothing to read yet: the column is there, invisible,
    // so the box can stand in the middle of the page.
    readonly empty: boolean;
    // How many times the page has sent the view to the foot: a branch the
    // person just made opens there, above the box. A count rather than a
    // flag, so the second request is heard as well as the first.
    readonly rejoined: number;
    readonly children: Snippet;
  }

  const { empty, rejoined, children }: Props = $props();

  const { lang } = ui();

  let anchoring = $state<Anchoring>("follow");
  let scroller = $state<HTMLDivElement | undefined>(undefined);
  let column = $state<HTMLDivElement | undefined>(undefined);
  // How many blocks stood in the column when the person left the foot,
  // and how many stand there now.
  let blocks = $state(0);
  let left = $state<number | null>(null);
  const unread = $derived(left === null ? 0 : Math.max(0, blocks - left));

  function count(): number {
    return column?.querySelectorAll("[data-wear]").length ?? 0;
  }

  // A selection inside the thread is something the person is doing; the
  // view does not pull it away.
  function selecting(): boolean {
    const selection = document.getSelection();
    return selection !== null && !selection.isCollapsed && column?.contains(selection.anchorNode) === true;
  }

  function toFoot(): void {
    const box = scroller;
    if (box !== undefined) box.scrollTop = box.scrollHeight;
  }

  function follow(): void {
    anchoring = "follow";
    toFoot();
  }

  let answered = untrack(() => rejoined);
  $effect(() => {
    if (rejoined === answered) return;
    answered = rejoined;
    untrack(follow);
  });

  $effect(() => {
    if (anchoring === "follow") left = null;
    else left ??= blocks;
  });

  $effect(() => {
    const held = column;
    if (held === undefined || scroller === undefined) return;
    // A room opens at its newest words; only the person's own scroll
    // moves it to reading back.
    anchoring = anchorAt({ kind: "opened" });
    // This write fires the column's scroll event, which measures the foot
    // again and agrees with follow; the second measurement is expected.
    toFoot();
    blocks = count();
    const watcher = new ResizeObserver(() => {
      blocks = count();
      if (anchoring === "follow" && !selecting()) toFoot();
    });
    watcher.observe(held);
    return () => {
      watcher.disconnect();
    };
  });
</script>

<div class="relative min-h-0 flex-1 -mx-wide narrow:mx-0">
  <div
    bind:this={scroller}
    data-thread=""
    tabindex="-1"
    class={[
      "h-full overflow-y-auto px-wide [scrollbar-width:none] narrow:px-0 narrow:[scrollbar-width:auto] [mask-image:linear-gradient(to_bottom,transparent_0,black_160px)] transition-opacity duration-panel",
      empty ? "opacity-0" : "",
    ]}
    onscroll={(event) => {
      anchoring = anchorAt({ kind: "scrolled", foot: footOf(event.currentTarget) });
    }}
  >
    <div bind:this={column} class="flex min-h-full w-full flex-col justify-end pt-section pb-section">
      {@render children()}
    </div>
  </div>
  {#if scroller !== undefined && column !== undefined && !empty}
    <Wear {scroller} {column} />
  {/if}
  {#if unread > 0}
    <button
      type="button"
      class="absolute bottom-snug left-1/2 inline-flex h-control-sm -translate-x-1/2 items-center gap-tight rounded-pill border border-edge-panel bg-raised px-base text-note text-text shadow-float hover:bg-raised-hover"
      aria-label={`${fill(say($lang, "talk_unread"), { n: String(unread) })} · ${say($lang, "talk_unread_follow")}`}
      onclick={() => {
        follow();
        // The button leaves with the count; the focus stays in the
        // column it was about rather than falling to the page.
        scroller?.focus({ preventScroll: true });
      }}
    >
      <Glyph name="chevron" size="sm" class="rotate-90" />
      {fill(say($lang, "talk_unread"), { n: String(unread) })}
    </button>
  {/if}
</div>
