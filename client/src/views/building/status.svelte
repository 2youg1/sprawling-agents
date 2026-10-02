<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // What this building has written since its last checkpoint, and where
  // its branch stands (client/Spec.lean §4-50). Every changed file is a way
  // out: pressing the row opens the file's working-tree text on the
  // right side, and "take back" returns it to what that checkpoint
  // holds. The checkpoint itself names the run, the room and the model,
  // and links into the run page - the way back from a line of code to
  // the session that wrote it.
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
  import { openDocument, rightItem } from "../inspect/open.svelte";
  import EmptyState from "../parts/empty.svelte";
  import TakeBack from "./take_back.svelte";

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

  // The row whose file is in front on the right side, marked the way a
  // chosen row is marked everywhere (docs/frontend-method.md §7B).
  function isOpen(path: string): boolean {
    const item = rightItem();
    return item !== null && item.kind === "document" && item.building === building && item.path === path;
  }
</script>

{#snippet checkpoint(commit: CommitAnswer)}
  <p class="flex flex-wrap items-baseline gap-x-base text-note">
    <span class="text-text-faint">{say($lang, "git_checkpoint")}</span>
    <a
      href={toFragment({ kind: "run", run: commit.run })}
      class="figure text-text-quiet hover:text-text">{shortOid(commit.oid)}</a
    >
    <a href={toFragment({ kind: "talk", address: commit.actor })} class="text-text-quiet hover:text-text"
      >{roomOf(commit.actor)}</a
    >
    {#if commit.model !== ""}
      <span class="text-text-faint">{commit.model}</span>
    {/if}
    {#if commit.spent > 0}
      <span class="figure text-text-faint">{fill(say($lang, "commits_spent"), { usd: usd(commit.spent) })}</span>
    {/if}
  </p>
{/snippet}

<div class="flex min-w-0 flex-col gap-base">
  <h2 class="text-note text-text-faint">{say($lang, "git_since_checkpoint")}</h2>
  {#if status === undefined}
    <p class="text-text-faint">…</p>
  {:else if status === null}
    <EmptyState missing="git_unavailable" />
  {:else}
    <p class="flex flex-wrap items-baseline gap-x-base text-note">
      <span class="text-text-faint">{say($lang, "git_branch")}</span>
      <span class="font-mono text-text-quiet">{status.branch ?? say($lang, "git_detached")}</span>
      <span class="text-text-faint">
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
      {@const point = status.checkpoint?.oid ?? null}
      <ul class="border-t border-edge text-note">
        {#each status.files as file (file.path)}
          <li class="relative flex min-w-0 items-center gap-snug border-b border-edge">
            {#if isOpen(file.path)}
              <span class="absolute inset-y-snug left-0 w-hair rounded-pill bg-accent" aria-hidden="true"></span>
            {/if}
            <button
              type="button"
              class={[
                "grid h-control min-w-0 flex-1 grid-cols-[16ch_minmax(0,1fr)_auto] narrow:grid-cols-[8ch_minmax(0,1fr)_auto] items-center gap-x-base rounded-control pr-snug pl-base text-left hover:wash",
                isOpen(file.path) ? "wash-strong" : "",
              ]}
              aria-label={fill(say($lang, "git_open_file"), { path: file.path })}
              onclick={() => {
                openDocument({ building, path: file.path, version: null });
              }}
            >
              <span class="truncate text-text-faint">{howWord($lang, file.how)}</span>
              <span class="truncate font-mono text-text">{file.path}</span>
              <span class="figure text-text-faint">{linesWord($lang, file.lines)}</span>
            </button>
            {#if point !== null}
              <TakeBack {building} path={file.path} {point} />
            {/if}
          </li>
        {/each}
      </ul>
    {:else}
      <EmptyState missing="git_clean" />
    {/if}
  {/if}
</div>
