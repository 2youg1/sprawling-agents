<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The preview of the version the editor stands on, a window at a time
  // (client-SPEC 4-26, 7N): `Query::Preview` reads one window into
  // blocks, and the next is asked from where that one ended once the
  // reader nears the bottom, or when the place carried over from the
  // source lies further on. A version's preview never goes stale, so
  // nothing here is asked twice. The draft is not in it - the city reads
  // versions, not drafts - and the line above says so while there is one.
  import { fill, say } from "../../core/lang";
  import { readAnswer } from "../../core/answered";
  import type { Positions } from "../../core/document_pos";
  import { ui } from "../../ui";
  import type { Address, Encoding, Laid as Window } from "../../wire";
  import { openDocument } from "../inspect/open.svelte";
  import Empty from "../parts/empty.svelte";
  import Unanswered from "../parts/unanswered.svelte";
  import Laid from "./laid.svelte";
  import { short } from "./reading";

  interface Props {
    readonly positions: Positions;
    readonly building: Address;
    readonly at: Address;
    readonly drafted: boolean;
    // The byte to open at, carried over from the source.
    readonly anchor: number | null;
    // The byte the first visible block starts at, as the reader scrolls.
    readonly onTop: (byte: number) => void;
  }

  const { positions, building, at, drafted, anchor, onTop }: Props = $props();

  const u = ui();
  const lang = u.lang;

  let windows = $state<readonly Window[]>([]);
  let unsupported = $state<Encoding | null>(null);
  let lost = $state<string | null>(null);
  let near = $state(true);
  let scroller = $state<HTMLDivElement>();
  let sentinel = $state<HTMLDivElement>();

  const total = $derived(positions.bytes(positions.editor.length));
  const reached = $derived(windows.at(-1)?.span.end ?? 0);
  const done = $derived(windows.length > 0 && reached >= total);
  const question = $derived({ preview: { version: positions.version, viewport: { start: reached, end: total } } });

  $effect(() => {
    if (done || unsupported !== null || lost !== null) return;
    if (!near && (anchor === null || anchor < reached)) return;
    const from = reached;
    return u.conn.asking.ask(question).subscribe((answer) => {
      const read = readAnswer(answer, (held) => ("preview" in held ? held.preview : undefined));
      if (read.kind === "unavailable") lost = read.query;
      if (read.kind !== "held") return;
      const preview = read.value.preview;
      if ("unsupported" in preview) unsupported = preview.unsupported.encoding;
      else if (preview.laid.span.start === from && (windows.at(-1)?.span.end ?? 0) === from) windows = [...windows, preview.laid];
    });
  });

  $effect(() => {
    const watched = sentinel;
    if (watched === undefined) return;
    const seen = new IntersectionObserver((entries) => {
      near = entries.some((entry) => entry.isIntersecting);
    }, { root: scroller ?? null, rootMargin: "600px" });
    seen.observe(watched);
    return () => {
      seen.disconnect();
    };
  });

  // The placed blocks, in document order, with the byte each starts at.
  function placed(): { readonly element: HTMLElement; readonly start: number }[] {
    return [...(scroller?.querySelectorAll<HTMLElement>("[data-start]") ?? [])].map((element) => ({
      element,
      start: Number(element.dataset.start ?? "0"),
    }));
  }

  // The carried place, once the window that holds it is drawn.
  $effect(() => {
    if (anchor === null || (anchor >= reached && !done)) return;
    const target = placed().filter((each) => each.start <= anchor).at(-1);
    target?.element.scrollIntoView({ block: "start" });
    onTop(target?.start ?? 0);
  });

  function scrolled(): void {
    const edge = scroller?.getBoundingClientRect().top ?? 0;
    const first = placed().find((each) => each.element.getBoundingClientRect().bottom > edge);
    if (first !== undefined) onTop(first.start);
  }

  // A relative link names a document beside this one; one that climbs
  // out of the building is no document of it.
  function open(target: string): void {
    const path = target.split(/[?#]/u)[0] ?? "";
    const parts = `${at.slice(building.length + 1, at.lastIndexOf("/") + 1)}${path}`.split("/");
    const walked: string[] = [];
    for (const part of parts) {
      if (part === "..") {
        if (walked.pop() === undefined) return;
      } else if (part !== "." && part !== "") walked.push(part);
    }
    if (walked.length > 0) openDocument({ building, path: walked.join("/"), version: null });
  }
</script>

<div class="min-h-0 flex-1 overflow-y-auto" bind:this={scroller} onscroll={scrolled}>
  {#if drafted}
    <p class="refrain-line text-note text-text-faint">
      {fill(say($lang, "refrain_preview_draft"), { version: short(positions.version) })}
    </p>
  {/if}
  {#if unsupported !== null}
    <p class="p-pane text-note text-text-quiet">{fill(say($lang, "refrain_preview_unsupported"), { encoding: unsupported })}</p>
  {:else if lost !== null}
    <div class="p-pane"><Unanswered query={lost} asked={question} /></div>
  {:else if done && windows.every((each) => each.blocks.length === 0)}
    <Empty missing="refrain_preview_empty" />
  {:else}
    <div class="mx-auto w-full max-w-measure px-pane py-base">
      {#each windows as window (window.span.start)}
        <Laid blocks={window.blocks} onOpen={open} placed />
      {/each}
      <div bind:this={sentinel} class="h-px"></div>
    </div>
  {/if}
</div>
