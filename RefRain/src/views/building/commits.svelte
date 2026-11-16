<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  
  
  import type { Attachment } from "svelte/attachments";
  import { SvelteSet } from "svelte/reactivity";
  import { derived } from "svelte/store";
  import { Option, Schema } from "effect";

  import { readAnswer } from "../../core/answered";
  import type { Answered } from "../../core/answered";
  import { commitsQuery } from "../../core/asking";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import { GitOid } from "../../wire";
  import type { Address, Answer, CommitAnswer, CommitsAnswer, Seq } from "../../wire";
  import EmptyState from "../parts/empty.svelte";
  import Field from "../parts/field.svelte";
  import Unanswered from "../parts/unanswered.svelte";
  import Commit from "./commit.svelte";
  import Whose from "./whose.svelte";

  interface Props {
    readonly building: Address;
  }

  const { building }: Props = $props();

  const u = ui();
  const lang = u.lang;

  
  let befores = $state<readonly (Seq | null)[]>([null]);
  let open = $state<string | null>(null);
  
  let typed = $state("");
  let whose = $state<GitOid | null>(null);
  
  let refused = $state(false);

  interface Pages {
    readonly rows: readonly CommitAnswer[];
    readonly more: boolean;
    
    
    readonly newest: Answered<CommitsAnswer>;
    readonly edge: Seq | undefined;
  }

  
  function foldPages(answers: readonly (Answer | undefined)[]): Pages {
    const rows: CommitAnswer[] = [];
    const seen = new SvelteSet<string>();
    for (const answer of answers) {
      const page = answer !== undefined && "commits" in answer ? answer.commits : undefined;
      for (const commit of page?.commits ?? []) {
        if (!seen.has(commit.oid)) {
          seen.add(commit.oid);
          rows.push(commit);
        }
      }
    }
    const newest = readAnswer(answers.at(-1), (held) =>
      "commits" in held ? held.commits : undefined,
    );
    const tail = newest.kind === "held" ? newest.value : undefined;
    return { rows, more: tail?.more === true, newest, edge: tail?.commits.at(-1)?.seq };
  }

  const pagesStore = $derived(
    derived(
      befores.map((before) => u.conn.asking.ask(commitsQuery(building, before))),
      foldPages,
    ),
  );
  const pages = $derived($pagesStore);

  function grow(): void {
    if (!pages.more || pages.edge === undefined) return;
    const edge = pages.edge;
    befores = befores.at(-1) === edge ? befores : [...befores, edge];
  }

  
  const sentinel: Attachment<HTMLLIElement> = (node) => {
    const watcher = new IntersectionObserver((entries) => {
      if (entries.some((entry) => entry.isIntersecting)) grow();
    });
    watcher.observe(node);
    return () => {
      watcher.disconnect();
    };
  };

  function toggle(commit: CommitAnswer, last: boolean): void {
    open = open === commit.oid ? null : commit.oid;
    
    
    if (last) grow();
  }

  function holds(oid: string): boolean {
    return pages.rows.some((row) => row.oid === oid);
  }

  
  function openRow(oid: string): void {
    open = oid;
    requestAnimationFrame(() => {
      document.getElementById(`commit-${oid}`)?.scrollIntoView({ block: "nearest" });
    });
  }

  const typedOid = $derived(Option.getOrNull(Schema.decodeOption(GitOid)(typed.trim())));
</script>

<div class="flex min-w-0 flex-col gap-base">
  <div class="flex min-w-0 flex-wrap items-end justify-between gap-x-wide gap-y-snug">
    <h2 class="sr-only">{say($lang, "bld_commits")}</h2>
    
    <form
      class="w-[42ch] max-w-full min-w-0"
      onsubmit={(event) => {
        event.preventDefault();
        refused = typedOid === null;
        whose = typedOid;
      }}
    >
      <Field
        label={say($lang, "whose_label")}
        placeholder={say($lang, "whose_placeholder")}
        mono
        value={typed}
        onInput={(value) => {
          typed = value;
          refused = false;
        }}
        {...refused ? { error: say($lang, "whose_not_oid") } : {}}
      />
    </form>
  </div>
  {#if whose !== null}
    <div class="border-l border-edge-input pl-base">
      <Whose oid={whose} />
    </div>
  {/if}
  {#if pages.rows.length > 0}
    <ul class="border-t border-edge" aria-label={say($lang, "bld_commits")}>
      {#each pages.rows as commit, index (commit.oid)}
        {@const last = !pages.more && index === pages.rows.length - 1}
        <Commit
          {building}
          {commit}
          older={pages.rows[index + 1] ?? null}
          first={last}
          open={open === commit.oid}
          onToggle={() => {
            toggle(commit, index === pages.rows.length - 1);
          }}
          {holds}
          onOpen={openRow}
        />
      {/each}
      <li {@attach sentinel} class="h-hair" aria-hidden="true"></li>
    </ul>
    <p class="text-note text-text-faint">
      {pages.more ? say($lang, "commits_more") : say($lang, "commits_all")}
    </p>
  {:else if pages.newest.kind === "held"}
    <EmptyState missing="commits_empty" />
  {:else if pages.newest.kind === "asking"}
    <p class="text-text-faint">…</p>
  {/if}
  {#if pages.newest.kind === "unavailable"}
    <Unanswered query={pages.newest.query} asked={commitsQuery(building, befores.at(-1) ?? null)} />
  {/if}
</div>
