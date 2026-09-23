<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // Draws what `core/prose` read: blocks and inline marks as elements,
  // never as HTML. Text nodes only, so a model's words cannot become
  // markup.
  //
  // **Width is the caller's.** The same reading draws a model's reply
  // inside the talk thread (the `talk` tier) and a long document inside
  // the file view (the `measure` tier), and the tier belongs to the
  // screen that chose the container (client-SPEC 4-33). What this file
  // holds is the one rule the two seats share: a table or a code block
  // never wraps per character - it scrolls inside its own box.
  import { blocks, inline as inlineOf } from "../core/prose";
  import type { Inline } from "../core/prose";

  interface Props {
    readonly text: string;
  }

  const { text }: Props = $props();

  const parsed = $derived(blocks(text));
</script>

{#snippet inlines(parts: readonly Inline[])}
  {#each parts as part (part)}
    {#if part.kind === "text"}
      {part.text}
    {:else if part.kind === "code"}
      <code class="rounded-control bg-raised px-tight font-mono text-note">{part.text}</code>
    {:else if part.kind === "strong"}
      <strong class="font-label">{part.text}</strong>
    {:else if part.kind === "em"}
      <em>{part.text}</em>
    {:else if part.kind === "link"}
      <a href={part.href} rel="noreferrer" target="_blank" class="text-accent underline">{part.text}</a>
    {/if}
  {/each}
{/snippet}

<div class="prose-region">
  {#each parsed as block (block)}
    {#if block.kind === "paragraph"}
      <p class="my-snug leading-relaxed">
        <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
        {@render inlines(block.inline)}
      </p>
    {:else if block.kind === "heading"}
      <!-- A document's own headings stay paragraphs in the page's own
           heading scale: one screen has one `h1`, and a file's `# Title`
           must not take it (ux-upgrades A12). -->
      <p class={["mt-base mb-tight font-heading", block.level <= 2 ? "text-heading" : "text-body"]}>
        <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
        {@render inlines(block.inline)}
      </p>
    {:else if block.kind === "list"}
      <ul class={["my-snug pl-wide", block.ordered ? "list-decimal" : "list-disc"]}>
        {#each block.items as item (item)}
          <li class="my-tight leading-relaxed">
            <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
            {@render inlines(item)}
          </li>
        {/each}
      </ul>
    {:else if block.kind === "code"}
      <pre class="my-snug overflow-x-auto rounded-card border border-edge bg-page p-base font-mono text-note leading-relaxed text-text-quiet">{block.text}</pre>
    {:else if block.kind === "quote"}
      <blockquote class="my-snug border-l-2 border-edge-input pl-base text-text-quiet">
        <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
        {@render inlines(block.inline)}
      </blockquote>
    {:else if block.kind === "table"}
      <div class="my-snug overflow-x-auto">
        <table class="w-full border-collapse text-note">
          <tbody>
            {#each block.rows as row, index (row)}
              {const cells = row.map((text) => ({ text }))}
              <tr class={index === 0 ? "text-text-quiet" : ""}>
                <!-- Each cell carries its own identity, so a row that
                     repeats a word keys no two cells alike (the pattern
                     `parts/code.svelte` settles its crumbs with). -->
                {#each cells as cell (cell)}
                  <td class="border-b border-edge px-snug py-tight align-top">
                    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
                    {@render inlines(inlineOf(cell.text))}
                  </td>
                {/each}
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  {/each}
</div>
