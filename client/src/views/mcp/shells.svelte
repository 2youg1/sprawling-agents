<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // How each shell interpreter's lines ended, from the whole ledger
  // (`Query::Shells`, wire D48): calls that ended with a code, and the
  // failures by class. The reading decides whether PowerShell 7 should
  // become the default (runtime D30); the counting and the classes are
  // `runtime::ShellTally`'s, so this part only draws them. A class this
  // page has no words for is written as the city spelled it.
  import type { Key } from "../../core/lang";
  import type { ShellCalls } from "../../wire";

  const CLASSES: readonly (readonly [string, Key])[] = [
    ["command_not_found", "shells_command_not_found"],
    ["syntax", "shells_syntax"],
    ["encoding", "shells_encoding"],
  ];

  export interface ShellLine {
    readonly interpreter: string;
    readonly calls: number;
    readonly failures: readonly (readonly [string, number])[];
  }

  export function lineOf(row: ShellCalls): ShellLine {
    return { interpreter: row.interpreter, calls: row.calls, failures: Object.entries(row.failures) };
  }

  export function classKey(spelled: string): Key | undefined {
    return CLASSES.find(([name]) => name === spelled)?.[1];
  }
</script>

<script lang="ts">
  import { readAnswer } from "../../core/answered";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Query } from "../../wire";
  import { Table, type Column } from "../parts/table";
  import Unanswered from "../parts/unanswered.svelte";

  const u = ui();
  const lang = u.lang;
  const question: Query = "shells";
  const asked = u.conn.asking.ask(question);
  const read = $derived(readAnswer($asked, (held) => ("shells" in held ? held.shells.interpreters.map(lineOf) : undefined)));

  const columns: readonly Column<ShellLine>[] = $derived([
    { key: "interpreter", header: say($lang, "shells_interpreter"), render: renderInterpreter, min: "12ch" },
    {
      key: "calls",
      header: say($lang, "shells_calls"),
      render: renderCalls,
      compare: (a: ShellLine, b: ShellLine) => a.calls - b.calls,
      min: "figure",
    },
    { key: "failures", header: say($lang, "shells_failures"), render: renderFailures, min: "24ch" },
  ]);
</script>

{#snippet renderInterpreter(row: ShellLine)}
  <span class="font-mono text-text-quiet">{row.interpreter}</span>
{/snippet}

{#snippet renderCalls(row: ShellLine)}
  <span class="tabular-nums">{row.calls}</span>
{/snippet}

{#snippet renderFailures(row: ShellLine)}
  {#if row.failures.length === 0}
    <span class="text-text-faint">{say($lang, "shells_no_failures")}</span>
  {:else}
    <span class="flex flex-wrap gap-base">
      {#each row.failures as [spelled, times] (spelled)}
        {@const key = classKey(spelled)}
        <span><span class="text-text-faint">{key === undefined ? spelled : say($lang, key)}</span> <span class="tabular-nums">{times}</span></span>
      {/each}
    </span>
  {/if}
{/snippet}

{#snippet noRows()}
  <p class="text-note text-text-faint">{say($lang, "shells_none")}</p>
{/snippet}

<section class="flex min-w-0 flex-col gap-base">
  <h2 class="text-note text-text-faint">{say($lang, "shells_title")}</h2>
  {#if read.kind === "unavailable"}
    <Unanswered query={read.query} reason={read.reason} asked={question} />
  {:else if read.kind === "asking"}
    <p class="text-text-faint">…</p>
  {:else}
    <Table caption={say($lang, "shells_title")} {columns} rows={read.value} keyOf={(row: ShellLine) => row.interpreter} empty={noRows} />
  {/if}
</section>
