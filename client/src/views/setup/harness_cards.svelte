<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The harness cards: the official harnesses a subscription reaches
  // the city through, one card each (`crates/wire/Spec.lean` §8-52).
  //
  // **The roster and the commands are the city's** (`Query::Harnesses`,
  // read from `agent_protocols::harness`); this file draws them and
  // holds no list of its own, and `harnesses.svelte` asks. Each card
  // says what this computer has of the harness, what command starts it,
  // and where
  // the harness's own vendor says how to sign in: the person signs in
  // inside the harness, and the city never asks for that credential.
  // The state is one of three (`harnessOf`): the launcher is missing,
  // the harness is not installed or not signed in - with the directories
  // the city looked in - or it is ready, with where it was found.

  // The names the vendors give their harnesses.
  // wording-ok: product names, which no language translates
  const NAMES: Readonly<Record<string, string>> = {
    claude_code: "Claude Code",
    codex: "Codex",
    grok_build: "Grok Build",
    kimi_code: "Kimi Code",
    pi: "Pi",
  };
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { HarnessLine } from "../../wire";
  import Badge from "../parts/badge.svelte";
  import { harnessOf } from "./harnesses";

  interface Props {
    readonly lines: readonly HarnessLine[];
  }

  const { lines }: Props = $props();
  const { lang } = ui();
</script>

<ul class="flex flex-col gap-base">
  {#each lines as line (line.name)}
    {@const reading = harnessOf(line.state)}
    <li class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
      <div class="flex items-center gap-snug">
        <span class="text-label font-label text-text">{NAMES[line.name] ?? line.name}</span>
        <Badge text={say($lang, reading.key)} status={reading.status} />
      </div>
      {#if reading.said !== null}
        <span class="min-w-0 break-all font-mono text-note text-text-faint">{reading.said}</span>
      {/if}
      {#if reading.looked.length > 0}
        <div class="flex min-w-0 flex-col gap-tight text-note">
          <span class="text-text-quiet">{say($lang, "harness_looked")}</span>
          <ul class="flex min-w-0 flex-col">
            {#each reading.looked as path (path)}
              <li class="min-w-0 break-all font-mono text-text-faint">{path}</li>
            {/each}
          </ul>
        </div>
      {/if}
      <span class="font-mono text-note text-text-quiet">{line.launch.join(" ")}</span>
      <a href={line.docs} target="_blank" rel="noopener noreferrer" class="text-note text-accent underline">
        {say($lang, "harness_sign_in")}
      </a>
    </li>
  {/each}
</ul>
