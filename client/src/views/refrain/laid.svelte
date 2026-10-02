<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The blocks the city read out of a stretch of Markdown, drawn as
  // elements and never through `innerHTML` (client/Spec.lean §4-26,
  // `crates/documents/Spec.lean` D21). A formula is drawn by KaTeX
  // (`formula.svelte`, §4-64a). What the city read and this page does not
  // draw - HTML, front matter, nesting too deep, and a formula KaTeX
  // refuses - is its own source in the mono face, so "something is here that this page does
  // not draw" never reads as "nothing is here". A link's target was
  // judged by the city (D23); a relative one opens the document it
  // names on the right side, through `onOpen`.
  import type { Snippet } from "svelte";

  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Inked from "../parts/inked.svelte";
  import Formula from "./formula.svelte";
  import Laid from "./laid.svelte";
  import type { Block, Inline } from "../../wire";

  interface Props {
    readonly blocks: readonly Block[];
    // Opens a link relative to the document; absent where links cannot
    // be followed - a reply has no document a relative link could be
    // relative to, so its link is drawn as its words (client/Spec.lean §4-53).
    readonly onOpen?: ((target: string) => void) | undefined;
    // Whether these are a window's top-level blocks, which carry the
    // byte they start at so a reading can find its place among them.
    readonly placed?: boolean;
  }

  const { blocks, onOpen, placed = false }: Props = $props();
  const { lang } = ui();

  const ABSOLUTE = /^(?:https?|mailto):/iu;

  function spanOf(block: Block): number {
    if ("heading" in block) return block.heading.span.start;
    if ("paragraph" in block) return block.paragraph.span.start;
    if ("list" in block) return block.list.span.start;
    if ("quote" in block) return block.quote.span.start;
    if ("code" in block) return block.code.span.start;
    if ("table" in block) return block.table.span.start;
    if ("rule" in block) return block.rule.span.start;
    if ("footnote" in block) return block.footnote.span.start;
    return block.unsupported.span.start;
  }

  const SPACE = " ";

  // One inline mark, read once into a shape the template can switch on.
  type Piece =
    | { readonly kind: "space" | "break" }
    | { readonly kind: "text" | "code" | "image" | "note" | "formula" | "raw"; readonly text: string }
    | { readonly kind: "emphasis" | "strong" | "strikethrough"; readonly parts: readonly Inline[] }
    | { readonly kind: "link"; readonly target: string; readonly parts: readonly Inline[] };

  function pieceOf(part: Inline): Piece {
    if (part === "soft_break") return { kind: "space" };
    if (part === "line_break") return { kind: "break" };
    if ("text" in part) return { kind: "text", text: part.text };
    if ("code" in part) return { kind: "code", text: part.code };
    if ("emphasis" in part) return { kind: "emphasis", parts: part.emphasis };
    if ("strong" in part) return { kind: "strong", parts: part.strong };
    if ("strikethrough" in part) return { kind: "strikethrough", parts: part.strikethrough };
    if ("link" in part) return { kind: "link", target: part.link.target, parts: part.link.content };
    if ("image" in part) return { kind: "image", text: part.image.alt };
    if ("footnote_reference" in part) return { kind: "note", text: part.footnote_reference.name };
    const { construct, source } = part.unsupported;
    return { kind: construct === "math" ? "formula" : "raw", text: source };
  }

  // The checker types a `{#snippet}` name as a void call, which the lint
  // lane rejects inside a render tag; the name is taken again as its
  // `Snippet` type, and the template renders that (as `parts/button.svelte`).
  const drawn: Snippet<[readonly Inline[]]> = inlines;

  const ALIGN: Record<string, string> = { none: "text-left", left: "text-left", center: "text-center", right: "text-right" };
</script>

{#snippet inlines(parts: readonly Inline[])}
  {#each parts.map(pieceOf) as piece, index (index)}
    {#if piece.kind === "space"}
      {SPACE}
    {:else if piece.kind === "break"}
      <br />
    {:else if piece.kind === "text"}
      {piece.text}
    {:else if piece.kind === "code"}
      <code class="rounded-control bg-raised px-tight font-mono text-note">{piece.text}</code>
    {:else if piece.kind === "emphasis"}
      <em>{@render drawn(piece.parts)}</em>
    {:else if piece.kind === "strong"}
      <strong class="font-label">{@render drawn(piece.parts)}</strong>
    {:else if piece.kind === "strikethrough"}
      <s>{@render drawn(piece.parts)}</s>
    {:else if piece.kind === "link"}
      {@const target = piece.target}
      {#if ABSOLUTE.test(target)}
        <a href={target} rel="noreferrer" target="_blank" class="text-accent underline">{@render drawn(piece.parts)}</a>
      {:else if onOpen === undefined}
        {@render drawn(piece.parts)}
      {:else}
        <button
          type="button"
          class="text-accent underline"
          onclick={() => {
            onOpen(target);
          }}>{@render drawn(piece.parts)}</button
        >
      {/if}
    {:else if piece.kind === "image"}
      <span class="text-text-faint">{piece.text}</span>
    {:else if piece.kind === "note"}
      <sup class="text-text-faint">{piece.text}</sup>
    {:else if piece.kind === "formula"}
      <Formula source={piece.text} />
    {:else if piece.kind === "raw"}
      <code class="font-mono text-note text-text-faint">{piece.text}</code>
    {/if}
  {/each}
{/snippet}

{#each blocks as block, index (index)}
  {@const start = placed ? spanOf(block) : undefined}
  {#if "heading" in block}
    <p data-start={start} class={["mt-base mb-tight font-heading", block.heading.level <= 2 ? "text-heading" : "text-body"]}>
      {@render drawn(block.heading.inline)}
    </p>
  {:else if "paragraph" in block}
    <p data-start={start} class="my-snug leading-relaxed">{@render drawn(block.paragraph.inline)}</p>
  {:else if "list" in block}
    {@const order = block.list.order}
    <svelte:element
      this={order === "bullet" ? "ul" : "ol"}
      data-start={start}
      start={order === "bullet" ? undefined : order.ordered.start}
      class={["my-snug pl-wide", order === "bullet" ? "list-disc" : "list-decimal"]}
    >
      {#each block.list.items as item, at (at)}
        <!-- A tight list's paragraphs sit on the item's own line; a task's
             box sits beside them, in place of the bullet. -->
        <li
          class={[
            "leading-relaxed",
            block.list.spacing === "loose" ? "my-snug" : "my-tight [&_p]:my-0",
            item.check === "not_a_task" ? "" : "-ml-wide flex list-none items-baseline gap-snug",
          ]}
        >
          {#if item.check !== "not_a_task"}
            <input
              type="checkbox"
              disabled
              checked={item.check === "done"}
              aria-label={say($lang, item.check === "done" ? "refrain_task_done" : "refrain_task_open")}
              class="w-glyph-sm shrink-0"
            />
          {/if}
          <div class="min-w-0 flex-1"><Laid blocks={item.blocks} {onOpen} /></div>
        </li>
      {/each}
    </svelte:element>
  {:else if "quote" in block}
    <blockquote data-start={start} class="my-snug border-l-2 border-edge-input pl-base text-text-quiet">
      <Laid blocks={block.quote.blocks} {onOpen} />
    </blockquote>
  {:else if "code" in block}
    <pre data-start={start} class="my-snug overflow-x-auto rounded-card border border-edge bg-page p-base font-mono text-note leading-relaxed text-text-quiet"><Inked text={block.code.text} source={block.code.info} /></pre>
  {:else if "table" in block}
    <div data-start={start} class="my-snug overflow-x-auto">
      <table class="w-full border-collapse text-note">
        <thead>
          <tr class="text-text-quiet">
            {#each block.table.head.cells as cell, at (at)}
              <th class={["border-b border-edge-input px-snug py-tight font-label", ALIGN[block.table.align[at] ?? "none"]]}>{@render drawn(cell)}</th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#each block.table.body as row, line (line)}
            <tr>
              {#each row.cells as cell, at (at)}
                <td class={["border-b border-edge px-snug py-tight align-top", ALIGN[block.table.align[at] ?? "none"]]}>{@render drawn(cell)}</td>
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else if "rule" in block}
    <hr data-start={start} class="my-base border-edge" />
  {:else if "footnote" in block}
    <div data-start={start} class="my-snug flex items-baseline gap-snug text-note text-text-quiet">
      <sup class="text-text-faint">{block.footnote.name}</sup>
      <div class="min-w-0 flex-1"><Laid blocks={block.footnote.blocks} {onOpen} /></div>
    </div>
  {:else}
    <pre data-start={start} class="my-snug overflow-x-auto whitespace-pre-wrap font-mono text-note text-text-faint">{block.unsupported.source}</pre>
  {/if}
{/each}
