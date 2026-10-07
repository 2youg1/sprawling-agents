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

**Coming back to a room comes back to the line.** Where the view stood
is kept per place within this tab (`standing.ts`): a room left while
reading back opens there again, once its thread is tall enough to hold
that line, and a room left at its foot follows its foot (refrain §3-14,
the second row; client/Spec.lean §4-63).

A font arriving, a code block taking its colours or a reply closing its
Markdown changes the height of text already there; in the held state the
engine's own scroll anchoring keeps the paragraph under the eye, and in
the following state the view is at the foot either way. -->
<script lang="ts">
  import { untrack } from "svelte";
  import type { Snippet } from "svelte";

  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import { anchorAt, footOf } from "./anchoring";
  import type { Anchoring } from "./anchoring";
  import { keepReading, readingAt } from "./standing";
  import Unread from "./unread.look.svelte";
  import Wear from "./wear.svelte";

  interface Props {
    // The room has nothing to read yet: the column is there, invisible,
    // so the box can stand in the middle of the page.
    readonly empty: boolean;
    // How many times the page has sent the view to the foot: a branch the
    // person just made opens there, above the box. A count rather than a
    // flag, so the second request is heard as well as the first.
    readonly rejoined: number;
    // The room the column reads, whose reading position it keeps.
    readonly place: string;
    readonly children: Snippet;
  }

  const { empty, rejoined, place, children }: Props = $props();

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

  // A line to return to that the thread is not yet tall enough to show:
  // each growth tries again, and nothing is kept until it is reached.
  let pending: number | null = null;

  function reach(): void {
    const box = scroller;
    if (pending === null || box === undefined) return;
    box.scrollTop = pending;
    if (box.scrollTop >= pending - 1) pending = null;
  }

  // The person's own hand on the column ends a return still waiting.
  function letGo(): void {
    pending = null;
  }

  // Where the view stands on arriving at a place: the line it was left
  // at, or the foot.
  function arrive(at: string): void {
    const top = readingAt(at);
    if (top === null) {
      pending = null;
      anchoring = anchorAt({ kind: "opened" });
      toFoot();
      return;
    }
    anchoring = "hold";
    pending = top;
    reach();
  }

  let standing = untrack(() => place);
  $effect(() => {
    const next = place;
    untrack(() => {
      if (next === standing) return;
      standing = next;
      arrive(next);
    });
  });

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
    // A room opens at its newest words, or at the line this tab left it
    // at; only the person's own scroll moves it to reading back. Going to
    // the foot fires the column's scroll event, which measures the foot
    // again and agrees with follow; the second measurement is expected.
    untrack(() => {
      arrive(place);
    });
    blocks = count();
    const watcher = new ResizeObserver(() => {
      blocks = count();
      reach();
      if (anchoring === "follow" && !selecting()) toFoot();
    });
    watcher.observe(held);
    const box = scroller;
    const hands = ["wheel", "pointerdown", "keydown"] as const;
    for (const hand of hands) box.addEventListener(hand, letGo, { passive: true });
    return () => {
      watcher.disconnect();
      for (const hand of hands) box.removeEventListener(hand, letGo);
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
      // The thread arrives with the first send and leaves when the room
      // empties: each way takes its own curve (docs/frontend-method.md §4-43).
      empty ? "opacity-0 ease-leave" : "ease-arrive",
    ]}
    onscroll={(event) => {
      if (pending !== null) return;
      anchoring = anchorAt({ kind: "scrolled", foot: footOf(event.currentTarget) });
      keepReading(place, anchoring === "hold" ? event.currentTarget.scrollTop : null);
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
    <!-- The button leaves with the count; the focus stays in the column
    it was about rather than falling to the page. -->
    <div class="absolute bottom-snug left-1/2 -translate-x-1/2">
      <Unread
        text={fill(say($lang, "talk_unread"), { n: String(unread) })}
        wire={{
          type: "button",
          "aria-label": `${fill(say($lang, "talk_unread"), { n: String(unread) })} · ${say($lang, "talk_unread_follow")}`,
          onclick: () => {
            follow();
            scroller?.focus({ preventScroll: true });
          },
        }}
      />
    </div>
  {/if}
</div>
