

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
    
    
    readonly empty: boolean;
    
    
    readonly rejoined: number;
    
    readonly place: string;
    readonly children: Snippet;
  }

  const { empty, rejoined, place, children }: Props = $props();

  const { lang } = ui();

  let anchoring = $state<Anchoring>("follow");
  let scroller = $state<HTMLDivElement | undefined>(undefined);
  let column = $state<HTMLDivElement | undefined>(undefined);
  
  
  let blocks = $state(0);
  let left = $state<number | null>(null);
  const unread = $derived(left === null ? 0 : Math.max(0, blocks - left));

  function count(): number {
    return column?.querySelectorAll("[data-wear]").length ?? 0;
  }

  
  function selecting(): boolean {
    const selection = document.getSelection();
    return (
      selection !== null &&
      !selection.isCollapsed &&
      column?.contains(selection.anchorNode) === true
    );
  }

  function toFoot(): void {
    const box = scroller;
    if (box !== undefined) box.scrollTop = box.scrollHeight;
  }

  function follow(): void {
    anchoring = "follow";
    toFoot();
  }

  
  let pending: number | null = null;

  function reach(): void {
    const box = scroller;
    if (pending === null || box === undefined) return;
    box.scrollTop = pending;
    if (box.scrollTop >= pending - 1) pending = null;
  }

  
  function letGo(): void {
    pending = null;
  }

  
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
    <div
      bind:this={column}
      class="flex min-h-full w-full flex-col justify-end pt-section pb-section"
    >
      {@render children()}
    </div>
  </div>
  {#if scroller !== undefined && column !== undefined && !empty}
    <Wear {scroller} {column} />
  {/if}
  {#if unread > 0}
    
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
