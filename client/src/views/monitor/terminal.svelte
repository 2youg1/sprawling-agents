<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The terminal record: every call the run made, in the order it made
// them. An exec call is drawn the way a terminal would have shown it -
// the command, what it wrote, and how it ended - with stderr under
// stdout in the alert ink, because a person scanning for what went
// wrong looks for that colour first. Any other call is its tool, what
// it was asked and what it answered, so the record has no gaps where
// the agent did something other than run a command.
</script>

<script lang="ts">
  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import Glyph from "../parts/glyph.svelte";
  import type { GlyphName } from "../parts/glyph";
  import { INK, rows } from "./ansi";
  import type { Span } from "./ansi";
  import type { Ending, Entry } from "./trace";

  interface Props {
    readonly entries: readonly Entry[];
  }

  const { entries }: Props = $props();
  const { lang } = ui();

  const MARK: Record<Ending["kind"], GlyphName> = { code: "check", stopped: "cross", running: "pulse", unread: "ring" };

  // Each stream cut at its line ends, the second one flagged so it is
  // drawn in the alert ink whatever colours it carried.
  function drawn(streams: readonly (readonly [string, boolean])[]): readonly { alert: boolean; row: readonly Span[] }[] {
    return streams.flatMap(([text, alert]) => rows(text).map((row) => ({ alert, row })));
  }

  function failed(ending: Ending): boolean {
    return (ending.kind === "code" && ending.code !== 0) || ending.kind === "stopped";
  }

  function said(ending: Ending): string {
    switch (ending.kind) {
      case "code":
        return fill(say($lang, "mon_exited"), { code: String(ending.code) });
      case "stopped":
        return fill(say($lang, "mon_stopped"), { why: ending.why });
      case "running":
        return say($lang, "mon_running");
      case "unread":
        return say($lang, "mon_unread");
    }
  }
</script>

{#if entries.length === 0}
  <p class="px-snug py-snug text-note text-text-faint">{say($lang, "mon_nothing_ran")}</p>
{/if}
{#each entries as entry (entry.at)}
  <div data-at={entry.at} class="border-b border-edge py-tight font-mono text-note leading-[1.6]">
    {#if entry.kind === "command"}
      <div class="flex min-h-control items-center gap-snug px-snug {failed(entry.ending) ? 'bg-alert/12' : ''}">
        <span class="shrink-0 text-text-faint" aria-hidden="true">$</span>
        <span class="min-w-0 truncate text-text">{entry.text}</span>
        <span
          class="ms-auto flex shrink-0 items-center gap-tight ps-snug {failed(entry.ending)
            ? 'text-alert'
            : entry.ending.kind === 'running'
              ? 'text-accent'
              : 'text-text-faint'}"
        >
          <Glyph name={MARK[entry.ending.kind]} size="sm" />
          <span>{said(entry.ending)}</span>
        </span>
      </div>
      <div class="px-snug">
        {#each drawn([[entry.stdout, false], [entry.stderr, true]]) as line, r (r)}
          <div class="ps-[2ch] -indent-[2ch] whitespace-pre-wrap wrap-break-word">{#each line.row as piece, p (p)}<span
                class="{line.alert ? 'text-alert' : INK[piece.tone]} {piece.bold ? 'font-semibold' : ''}">{piece.text}</span
              >{/each}</div>
        {/each}
        {#if entry.cut > 0}
          <div class="text-text-faint">{fill(say($lang, "mon_lines_cut"), { n: String(entry.cut) })}</div>
        {/if}
      </div>
    {:else}
      <div class="flex min-h-control items-center gap-snug px-snug">
        <span class="shrink-0 text-accent">{entry.tool}</span>
        <span class="min-w-0 truncate text-text-quiet">{entry.subject}</span>
        {#if entry.outcome === "failed"}
          <Glyph name="cross" size="sm" class="ms-auto shrink-0 text-alert" />
        {/if}
      </div>
      <div class="px-snug text-text-faint">
        {#each drawn([[entry.asked, false], [entry.answered, entry.outcome === "failed"]]) as line, r (r)}
          <div class="ps-[2ch] -indent-[2ch] whitespace-pre-wrap wrap-break-word">{#each line.row as piece, p (p)}<span
                class="{line.alert ? 'text-alert' : INK[piece.tone]} {piece.bold ? 'font-semibold' : ''}">{piece.text}</span
              >{/each}</div>
        {/each}
      </div>
    {/if}
  </div>
{/each}
