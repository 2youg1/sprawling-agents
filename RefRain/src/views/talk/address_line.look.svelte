

<script lang="ts">
  
  
  import type { Snippet } from "svelte";
  import type { AddressLineLook, BoxWords, Lesson } from "./address_line";

  
  type Seat = { readonly at: "head" } | ({ readonly at: "box" } & BoxWords);

  interface Props extends AddressLineLook {
    readonly seat: Seat;
    readonly lesson?: Lesson | undefined;
  }

  const { seat, lesson, ...look }: Props = $props();

  
  const lessonOf: Snippet<[Lesson | undefined]> = taught;
  const button: Snippet<[string, Lesson | undefined]> = press;
</script>

{#snippet taught(lesson: Lesson | undefined)}
  {#if lesson !== undefined}
    
    <span class="min-w-0 shrink-10000 truncate text-text-faint" aria-hidden="true"
      >&emsp;{lesson.words}{#if lesson.chord !== ""}<span class="pointer-coarse:hidden"
          >&ensp;{lesson.chord}</span
        >{/if}</span
    >
  {/if}
{/snippet}

{#snippet press(size: string, within: Lesson | undefined)}
  <button
    {...look.wire}
    class={[
      "group relative flex max-w-full min-w-0 items-baseline rounded-control text-text-faint",
      size,
      "transition-[color,transform] duration-short ease-leave hover:text-text-quiet hover:ease-arrive focus-visible:text-text-quiet",
      "active:scale-(--scale-press) active:ease-arrive still:transition-none still:active:scale-100",
      "pointer-coarse:before:absolute pointer-coarse:before:top-1/2 pointer-coarse:before:left-1/2 pointer-coarse:before:size-touch pointer-coarse:before:-translate-1/2 pointer-coarse:before:content-['']",
    ]}
  >
    {#if look.building !== ""}
      <span class="min-w-0 shrink-1000 truncate">{look.building}</span>
    {/if}
    <span class="min-w-0 truncate text-text-quiet group-hover:text-text">{look.room}</span>
    {#if look.session !== undefined}
      
      <span class="shrink-0 px-tight" aria-hidden="true">·</span>
      <span class="figure shrink-0">{look.session}</span>
    {/if}
    {@render lessonOf(within)}
  </button>
{/snippet}

{#if seat.at === "head"}
  <header aria-label={look.head} class="fade flex h-control shrink-0 items-center">
    {@render button("text-note", lesson)}
  </header>
{:else}
  <span class="flex max-w-full min-w-0 items-baseline">
    <span class="shrink-0 whitespace-pre" aria-hidden="true">{seat.before}</span>
    <span class="pointer-events-auto flex min-w-0">{@render button("", undefined)}</span>
    <span class="shrink-0 whitespace-pre" aria-hidden="true">{seat.after}</span>
    {@render lessonOf(lesson)}
  </span>
{/if}
