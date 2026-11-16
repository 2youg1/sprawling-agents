<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import type { ListLook, PopoverLook, RowLook } from "../parts/popover_wiring";
  import { SECTION } from "./picker";
  import type { PickerMenu, SectionLook } from "./picker_look";
  interface Props {
    readonly look: PopoverLook;
    readonly menu: PickerMenu;
  }

  const { look, menu }: Props = $props();

  const QUIET: SectionLook = {
    heading: "",
    aside: undefined,
    spare: 0,
    band: false,
    scrolls: false,
    empty: "",
    heard: {},
  };
  const HEADING =
    "flex h-control-sm items-center justify-between gap-base px-snug text-note text-text-faint";
  const ROW =
    "relative flex h-touch min-w-0 shrink-0 cursor-pointer items-center gap-snug rounded-control px-base text-body outline-1 -outline-offset-1 outline-transparent";
  const NOTE = "line-clamp-2 min-h-[2lh] px-snug text-note text-text-quiet";

  const tone = (item: RowLook): string[] => [
    item.chosen
      ? "wash-strong text-text"
      : item.cursor
        ? "bg-raised-hover text-text-quiet"
        : "text-text-quiet hover:bg-raised-hover hover:text-text",
    item.cursor ? "chosen" : "",
  ];

  const cap = $derived(
    look.room === null
      ? undefined
      : `min(85dvh, calc(${String(look.room)}px - var(--spacing-snug) * 2))`,
  );
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
    "absolute right-0 flex max-h-[85dvh] w-[34rem] max-w-full flex-col gap-snug overflow-y-auto rounded-panel border border-edge-panel bg-raised py-snug pl-snug shadow-float",
    look.side === "above" ? "bottom-full mb-snug rise" : "top-full mt-snug drop",
  ]}
  style:max-height={cap}
  {...look.dialog}
>
  {#each look.lists as list (list.key)}
    {@const section = menu.sections[list.key] ?? QUIET}
    {#if list.key === SECTION.first}
      <input
        class="h-touch w-full min-w-0 shrink-0 rounded-control bg-page px-base text-body text-text outline-hidden placeholder:text-text-faint"
        {...menu.filter}
      />
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
          <li
            class={[
              ROW,
              section.band ? "flex-1 justify-center" : "justify-between",
              "transition-[background-color,color] ease-leave hover:ease-arrive active:wash-strong active:ease-arrive",
              ...tone(item),
            ]}
            aria-label={section.heard[item.row.id]}
            {...item.wire}
          >
            <span
              class={section.band || item.row.secondary === undefined
                ? "min-w-0 truncate"
                : "max-w-[60%] shrink-0 truncate"}>{item.row.label}</span
            >
            {#if item.row.secondary !== undefined && !section.band}
              <span class="min-w-0 truncate text-note text-text-faint">{item.row.secondary}</span>
            {/if}
          </li>
        {:else}
          <li
            role="presentation"
            class="flex h-touch items-center px-base text-note text-text-faint"
          >
            {section.empty}
          </li>
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
    <div class="invisible flex flex-col gap-tight" aria-hidden="true">
      <div class={HEADING}></div>
      <div class="h-touch"></div>
      <p class={NOTE}></p>
    </div>
  {/if}
</div>
