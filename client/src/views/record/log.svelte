<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The process log beside the record: what this machine was writing.
// The log arrives on its own class of frame and is kept in a window
// rather than paged: it is a diagnostic and not history, so a line that
// scrolled out of the window is gone and nothing is owed for it. Three
// filters, because the three fields a person narrows by are the run,
// the module and the level - and the first of those is what makes a
// busy city readable at all.

import type { Key } from "../../core/lang";
import type { LogLevel, LogLine, TimeMs } from "../../wire";

// The five levels `docs/logging.md` names, in the order it names them,
// and each one's word in the phrase table. A table rather than a
// spelled-out name so a key that `lang.json` drops fails to compile
// instead of failing to render.
const LEVELS: readonly LogLevel[] = ["refuse", "effect", "decide", "trace", "wire"];
const LEVEL_NAMES: Record<LogLevel, Key> = {
  refuse: "log_refuse",
  effect: "log_effect",
  decide: "log_decide",
  trace: "log_trace",
  wire: "log_wire",
};

// Every name that has spoken, in the order a person would look for
// them. Derived from what arrived rather than from a table: the modules
// that write lines are the modules worth offering as a filter.
function namesIn(
  lines: readonly LogLine[],
  of: (line: LogLine) => string | null | undefined,
): string[] {
  const seen: string[] = [];
  for (const line of lines) {
    const name = of(line);
    if (typeof name === "string" && !seen.includes(name)) seen.push(name);
  }
  return seen.sort();
}

// When this machine wrote a line, or nothing when its clock could not
// be read. The ledger position beside it is the anchor either way.
function wroteAt(line: LogLine): TimeMs | null {
  return line.t ?? null;
}

// Whether a filter value names one of the five levels. The empty value
// is "every one of them" and is not a level.
function isLevel(raw: string): raw is LogLevel {
  return LEVELS.some((each) => each === raw);
}
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { clock, hhmmss } from "../../core/time";
  import { ui } from "../../ui";
  import EmptyState from "../parts/empty.svelte";
  import Tip from "../parts/tip.svelte";

  const u = ui();
  const lang = u.lang;
  const belief = u.conn.belief;

  let level = $state<LogLevel | "">("");
  let moduleName = $state("");
  let runName = $state("");

  const lines = $derived($belief.logs);
  const modules = $derived(namesIn(lines, (line) => line.module));
  const runs = $derived(namesIn(lines, (line) => line.run));
  const shown = $derived(
    lines.filter(
      (line) =>
        (level === "" || line.level === level) &&
        (moduleName === "" || line.module === moduleName) &&
        (runName === "" || String(line.run ?? "") === runName),
    ),
  );

  // One dropdown. The empty value is "every one of them", which is the
  // state a person opens this page in. The three fields a person
  // narrows by are the run, the module and the level.
</script>

{#snippet narrow(props: {
  label: string;
  every: string;
  options: readonly string[];
  current: string;
  onPick: (value: string) => void;
})}
  <label class="flex items-center gap-tight text-note text-text-faint">
    {props.label}
    <select
      class="rounded-control bg-chrome px-snug py-tight font-mono text-note text-text"
      value={props.current}
      onchange={(event) => {
        props.onPick(event.currentTarget.value);
      }}
    >
      <option value="">{props.every}</option>
      {#each props.options as each (each)}
        <option value={each}>{each}</option>
      {/each}
    </select>
  </label>
{/snippet}

<div class="flex flex-col gap-base">
  <div class="flex flex-wrap items-center gap-base">
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
    {@render narrow({
      label: say($lang, "log_levels"),
      every: say($lang, "log_every"),
      options: LEVELS,
      current: level,
      onPick: (value) => {
        level = isLevel(value) ? value : "";
      },
    })}
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
    {@render narrow({
      label: say($lang, "log_modules"),
      every: say($lang, "log_every"),
      options: modules,
      current: moduleName,
      onPick: (value) => {
        moduleName = value;
      },
    })}
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
    {@render narrow({
      label: say($lang, "log_runs"),
      every: say($lang, "log_every"),
      options: runs,
      current: runName,
      onPick: (value) => {
        runName = value;
      },
    })}
    <span class="flex-1"></span>
    <span class="text-note text-text-faint">{say($lang, "log_window")}</span>
  </div>
  {#if shown.length === 0}
    <EmptyState missing="log_empty" />
  {:else}
    <ul class="font-mono text-note">
      {#each [...shown].reverse() as line (line.seq)}
        {@const at = wroteAt(line)}
        <li class="settled-row flex gap-base border-b border-edge py-tight">
          <span class="w-figure shrink-0 text-right text-text-faint">{line.seq}</span>
          <span class="w-figure shrink-0 whitespace-nowrap text-text-faint">
            {#if at !== null}
              <Tip text={clock($lang, at)}>
                {#snippet children(hint)}
                  <!-- The column shows the time of day; the day itself
                  is one key away rather than one hover away. -->
                  <!-- svelte-ignore a11y_no_noninteractive_tabindex (the hint must be reachable by keyboard) -->
                  <span tabindex="0" aria-describedby={hint}>{hhmmss(at)}</span>
                {/snippet}
              </Tip>
            {/if}
          </span>
          <span class="shrink-0 text-text">{say($lang, LEVEL_NAMES[line.level])}</span>
          <span class="shrink-0 text-text-faint">{line.module}</span>
          <!-- One line of the process log is one line: a log row is
          read as a row of a window, and a message that wraps turns a
          scannable column into a paragraph. The ledger holds the same
          event whole. -->
          <span class="min-w-0 flex-1 truncate text-text-quiet">{line.line}</span>
        </li>
      {/each}
    </ul>
  {/if}
</div>
