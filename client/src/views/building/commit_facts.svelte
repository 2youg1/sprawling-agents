<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // What one commit is, as a sheet of facts (client/Spec.lean §4-50, UC2): the
  // run that wrote it, the session and the model, what that run cost,
  // the runs it succeeded, its parents and its whole oid. A parent the
  // page already holds opens its own row; one it does not is asked for
  // by its oid and drawn here, under the parents, which is the one place
  // the question "who wrote this commit" is answered (`Query::Commit`).
  import { say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { usd } from "../../core/time";
  import { ui } from "../../ui";
  import type { CommitAnswer, Effort, GitOid } from "../../wire";
  import { shortOid } from "../changes";
  import Copy from "../machine/copy.svelte";
  import Whose from "./whose.svelte";

  interface Props {
    readonly commit: CommitAnswer;
    // Whether a parent is a row the page already holds, and the way to
    // open that row; an absent hand means every parent is asked for.
    readonly holds?: ((oid: GitOid) => boolean) | undefined;
    readonly onOpen?: ((oid: GitOid) => void) | undefined;
  }

  const { commit, holds, onOpen }: Props = $props();

  const lang = ui().lang;

  // The parent asked for by its oid, drawn under the parents.
  let asked = $state<GitOid | null>(null);

  // How long a run id is when a line names a run rather than lists it.
  const RUN_SHORT = 8;

  function modelOf(model: string, effort: Effort | null | undefined): string {
    return effort === null || effort === undefined ? model : `${model} · ${effort}`;
  }

  function parent(oid: GitOid): void {
    if (holds?.(oid) === true) {
      onOpen?.(oid);
    } else {
      asked = asked === oid ? null : oid;
    }
  }
</script>

<div class="flex min-w-0 flex-col gap-base">
  <dl class="grid grid-cols-[repeat(auto-fill,minmax(22ch,1fr))] gap-x-gutter gap-y-base text-note">
    <div class="flex min-w-0 flex-col">
      <dt class="text-text-faint">{say($lang, "commit_run")}</dt>
      <dd>
        <a href={toFragment({ kind: "run", run: commit.run, lens: "changes" })} class="figure text-text hover:text-accent"
          >{commit.run.slice(0, RUN_SHORT)}</a
        >
      </dd>
    </div>
    {#if commit.session !== null && commit.session !== undefined}
      <div class="flex min-w-0 flex-col">
        <dt class="text-text-faint">{say($lang, "commit_session")}</dt>
        <dd class="truncate">
          <a href={toFragment({ kind: "talk", address: commit.actor })} class="text-text hover:text-accent"
            >{commit.session}</a
          >
        </dd>
      </div>
    {/if}
    {#if commit.model !== ""}
      <div class="flex min-w-0 flex-col">
        <dt class="text-text-faint">{say($lang, "commit_model")}</dt>
        <dd class="truncate text-text">{modelOf(commit.model, commit.effort)}</dd>
      </div>
    {/if}
    <div class="flex min-w-0 flex-col">
      <dt class="text-text-faint">{say($lang, "commit_spent")}</dt>
      <dd class="figure text-text">{usd(commit.spent)}</dd>
    </div>
    {#if commit.lineage.length > 1}
      <div class="flex min-w-0 flex-col">
        <dt class="text-text-faint">{say($lang, "commits_lineage")}</dt>
        <dd class="flex flex-wrap gap-x-snug">
          {#each commit.lineage.slice(1) as run (run)}
            <a href={toFragment({ kind: "run", run })} class="figure text-text-quiet hover:text-accent"
              >{run.slice(0, RUN_SHORT)}</a
            >
          {/each}
        </dd>
      </div>
    {/if}
    <div class="flex min-w-0 flex-col">
      <dt class="text-text-faint">{say($lang, "commit_parents")}</dt>
      <dd class="flex flex-wrap gap-x-snug">
        {#each commit.parents ?? [] as oid (oid)}
          <button
            type="button"
            class={["figure hover:text-accent", asked === oid ? "text-text" : "text-text-quiet"]}
            aria-expanded={holds?.(oid) === true ? undefined : asked === oid}
            onclick={() => {
              parent(oid);
            }}>{shortOid(oid)}</button
          >
        {:else}
          <span class="text-text-faint">{say($lang, "commit_root")}</span>
        {/each}
      </dd>
    </div>
    <div class="col-span-full flex min-w-0 items-center gap-base">
      <dt class="sr-only">{say($lang, "commit_oid")}</dt>
      <dd class="min-w-0 truncate font-mono text-text-faint">{commit.oid}</dd>
      <Copy text={commit.oid} />
    </div>
  </dl>
  {#if asked !== null}
    <div class="border-l border-edge-input pl-base">
      <Whose oid={asked} />
    </div>
  {/if}
</div>
