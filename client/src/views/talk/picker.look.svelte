<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- How the open model picker is drawn, and nothing else (client D95,
docs/frontend-method.md §7I): the popover's lists as the picker's
sections, top to bottom - recent combinations, the filter, the first
choice level, the second, the thinking band - each under its heading.
Every key, role and `aria-*` value arrives in a wire bag from the
popover seat or from `picker_look.ts`. What is applied reads as the
deeper wash; the cursor is the accent bar at the row's leading edge
(§7B). A section keeps the room of the largest state reachable without
typing, drawn as rows nobody can see, so a choice never moves a target
already on the screen; a list opened in full scrolls inside that room. -->
<script lang="ts">
  import type { ListLook, PopoverLook, RowLook } from "../parts/popover_wiring";
  import { SECTION } from "./picker";
  import type { PickerMenu, SectionLook } from "./picker_look";

  interface Props {
    readonly look: PopoverLook;
    readonly menu: PickerMenu;
  }

  const { look, menu }: Props = $props();

  const QUIET: SectionLook = { heading: "", aside: undefined, spare: 0, band: false, scrolls: false, empty: "", heard: {} };
  const HEADING = "flex h-control-sm items-center justify-between gap-base px-snug text-note text-text-faint";
  const ROW = "relative flex h-touch min-w-0 shrink-0 cursor-pointer items-center gap-snug rounded-control px-base text-body outline-1 -outline-offset-1 outline-transparent";
  const NOTE = "line-clamp-2 min-h-[2lh] px-snug text-note text-text-quiet";

  const tone = (item: RowLook): string[] => [
    item.chosen ? "wash-strong text-text" : item.cursor ? "bg-raised-hover text-text-quiet" : "text-text-quiet",
    item.cursor ? "chosen" : "",
  ];
  // The sentence under the band: the level the cursor is on, else the
  // level in use.
  const noteOf = (list: ListLook): string | undefined =>
    (list.rows.find((item) => item.cursor) ?? list.rows.find((item) => item.chosen))?.row.secondary;
</script>

{#snippet spareRows(count: number)}
  {#each { length: count } as _unused, at (at)}
    <li role="presentation" aria-hidden="true" class="h-touch shrink-0"></li>
  {/each}
{/snippet}

<div
  class={[
    // The window's height is the one cap: a picker taller than the
    // room a short window leaves scrolls rather than leaving the screen.
    // The scroller's reserved gutter stands in for the right padding.
    "absolute right-0 flex max-h-[85dvh] w-[34rem] max-w-full flex-col gap-snug overflow-y-auto rounded-panel border border-edge-panel bg-raised py-snug pl-snug shadow-float",
    look.side === "above" ? "bottom-full mb-snug rise" : "top-full mt-snug drop",
  ]}
  {...look.dialog}
>
  {#each look.lists as list (list.key)}
    {@const section = menu.sections[list.key] ?? QUIET}
    {#if list.key === SECTION.first}
      <input class="h-touch w-full min-w-0 shrink-0 rounded-control bg-page px-base text-body text-text outline-hidden placeholder:text-text-faint"
        {...menu.filter} />
    {/if}
    <div class="flex min-w-0 flex-col gap-tight">
      <div class={HEADING}>
        <span class="min-w-0 truncate">{section.heading}</span>
        {#if section.aside !== undefined}<span class="shrink-0">{section.aside}</span>{/if}
      </div>
      <ul
        class={[
          section.band ? "flex gap-tight" : "flex flex-col",
          section.scrolls && "max-h-[calc(var(--spacing-touch)*6)] overflow-y-auto",
        ]}
        {...list.wire}
        aria-label={section.heading}
      >
        {#each list.rows as item (item.key)}
          <li class={[ROW, section.band ? "flex-1 justify-center" : "justify-between", ...tone(item)]}
            aria-label={section.heard[item.row.id]} {...item.wire}>
            <!-- The name keeps its width and the facts beside it give
                 way, so a provider is never cut to a prefix. -->
            <span class={section.band || item.row.secondary === undefined ? "min-w-0 truncate" : "max-w-[60%] shrink-0 truncate"}>{item.row.label}</span>
            {#if item.row.secondary !== undefined && !section.band}
              <span class="min-w-0 truncate text-note text-text-faint">{item.row.secondary}</span>
            {/if}
          </li>
        {:else}
          <li role="presentation" class="flex h-touch items-center px-base text-note text-text-faint">{section.empty}</li>
        {/each}
        <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself) -->
        {@render spareRows(list.rows.length === 0 ? Math.max(0, section.spare - 1) : section.spare)}
      </ul>
      {#if section.band}
        <p class={NOTE}>{noteOf(list) ?? ""}</p>
      {/if}
    </div>
  {/each}
  {#if menu.bandRoom}
    <!-- The thinking band's room, kept for a reachable model that has
         one while the model chosen now has none. -->
    <div class="invisible flex flex-col gap-tight" aria-hidden="true">
      <div class={HEADING}></div>
      <div class="h-touch"></div>
      <p class={NOTE}></p>
    </div>
  {/if}
</div>
