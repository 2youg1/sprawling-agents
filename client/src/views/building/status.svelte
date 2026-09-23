<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // What this building has written since its last checkpoint, and where
  // its branch stands. The checkpoint itself is a row naming the run,
  // the room, the model and what that run cost, and it links into the
  // run page - which is the way back from a line of code to the session
  // that wrote it.
  //
  // The three states of the question are three states of the screen: a
  // city that has not answered yet says nothing rather than guessing,
  // one without git says so, and one with git gets the whole reading.

  import { fill, say } from "../../core/lang";
  import { roomOf, toFragment } from "../../core/route";
  import { usd } from "../../core/time";
  import { ui } from "../../ui";
  import type { Address, CommitAnswer, GitStatusAnswer } from "../../wire";
  import { howWord, linesWord, shortOid } from "../changes";

  interface Props {
    readonly building: Address;
  }

  const { building }: Props = $props();

  const u = ui();
  const lang = u.lang;

  const asked = $derived(u.conn.asking.ask({ git_status: { building } }));
  const status = $derived.by((): GitStatusAnswer | null | undefined => {
    const held = $asked;
    if (held === undefined) return undefined;
    return "git_status" in held ? held.git_status : null;
  });
</script>

{#snippet checkpoint(commit: CommitAnswer)}
  <p class="mb-base flex flex-wrap items-baseline gap-base text-note">
    <span class="text-text-quiet">{say($lang, "git_checkpoint")}</span>
    <a
      href={toFragment({ kind: "run", run: commit.run })}
      class="font-mono text-text-faint hover:text-text-quiet">{shortOid(commit.oid)}</a
    >
    <a
      href={toFragment({ kind: "talk", address: commit.actor })}
      class="text-text-faint hover:text-text-quiet">{roomOf(commit.actor)}</a
    >
    {#if commit.model !== ""}
      <span class="text-text-disabled">{commit.model}</span>
    {/if}
    {#if commit.spent > 0}
      <span class="text-text-disabled"
        >{fill(say($lang, "commits_spent"), { usd: usd(commit.spent) })}</span
      >
    {/if}
  </p>
{/snippet}

<div>
  <h2 class="mb-base text-heading font-heading">{say($lang, "bld_changes")}</h2>
  {#if status === undefined}
    <p class="text-text-disabled">…</p>
  {:else if status === null}
    <p class="text-text-faint">{say($lang, "git_unavailable")}</p>
  {:else}
    <p class="mb-base flex flex-wrap items-baseline gap-base text-note">
      <span class="text-text-quiet">{say($lang, "git_branch")}</span>
      <span class="font-mono text-text-faint">{status.branch ?? say($lang, "git_detached")}</span>
      <span class="text-text-disabled">
        {#if status.drift !== null && status.drift !== undefined}
          {fill(say($lang, "git_drift"), {
            ahead: String(status.drift.ahead),
            behind: String(status.drift.behind),
          })}
        {:else}
          {say($lang, "git_no_upstream")}
        {/if}
      </span>
    </p>
    {#if status.checkpoint !== null && status.checkpoint !== undefined}
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
      {@render checkpoint(status.checkpoint)}
    {/if}
    {#if status.files.length > 0}
      <ul class="text-note">
        {#each status.files as file (file.path)}
          <li class="flex items-center gap-base border-b border-edge py-snug">
            <span class="w-figure shrink-0 text-text-faint">{howWord($lang, file.how)}</span>
            <span class="flex-1 truncate font-mono text-text-quiet">{file.path}</span>
            <span class="shrink-0 font-mono text-text-disabled">{linesWord($lang, file.lines)}</span>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="text-text-faint">{say($lang, "git_clean")}</p>
    {/if}
  {/if}
</div>
