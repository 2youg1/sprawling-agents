<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The harness page: the official harnesses a subscription reaches
  // the city through, one card each (`crates/wire/Spec.lean` §8-52).
  //
  // **The roster and the commands are the city's** (`Query::Harnesses`,
  // read from `agent_protocols::harness`); this page draws them and holds no
  // list of its own. Each card says whether this computer can run the
  // command that starts the harness, what that command is, and where
  // the harness's own vendor says how to sign in: the person signs in
  // inside the harness, and the city never asks for that credential.

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
  import { QUERIES } from "../../core/asking";
  import { readAnswer } from "../../core/answered";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Badge from "../parts/badge.svelte";
  import Unanswered from "../parts/unanswered.svelte";

  const u = ui();
  const lang = u.lang;

  const asked = u.conn.asking.ask(QUERIES.harnesses);
  const read = $derived(readAnswer($asked, (held) => ("harnesses" in held ? held.harnesses.harnesses : undefined)));
</script>

{#if read.kind === "held"}
  <ul class="flex flex-col gap-base">
    {#each read.value as line (line.name)}
      <li class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
        <div class="flex items-center gap-snug">
          <span class="text-label font-label text-text">{NAMES[line.name] ?? line.name}</span>
          <!-- The launcher's presence, as before wire D23; the three states are drawn by the page that renders them. -->
          <Badge
            text={say($lang, "launcher_missing" in line.state ? "harness_missing" : "harness_found")}
            status={"launcher_missing" in line.state ? "idle" : "done"}
          />
        </div>
        <span class="font-mono text-note text-text-quiet">{line.launch.join(" ")}</span>
        <a href={line.docs} target="_blank" rel="noopener noreferrer" class="text-note text-accent underline">
          {say($lang, "harness_sign_in")}
        </a>
      </li>
    {/each}
  </ul>
{:else if read.kind === "unavailable"}
  <Unanswered query={read.query} asked={QUERIES.harnesses} />
{/if}
