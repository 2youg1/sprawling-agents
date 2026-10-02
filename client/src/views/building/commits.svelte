<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The commits a building made, newest first, and the way back from a
  // line of code to the session that wrote it (client-SPEC 4-50). Each
  // row is the commit as one line and opens into its facts and the
  // files it changed; scrolling to the end asks for the next page, and
  // the end of the list says whether it is the building's first commit
  // or a page still coming. The box at the head asks for any commit by
  // its oid - the command line's `whose`.
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
  import Unanswered from "../parts/unanswered.svelte";
  import Commit from "./commit.svelte";
  import Whose from "./whose.svelte";

  interface Props {
    readonly building: Address;
  }

  const { building }: Props = $props();

  const u = ui();
  const lang = u.lang;

  // One `before` per page asked; the first page has none. A page is
  // its own question, so an older page stays valid while the newest
  // one is re-asked after every commit (client-SPEC 4-15).
  let befores = $state<readonly (Seq | null)[]>([null]);
  let open = $state<string | null>(null);
  // The oid typed into the head's box, and the one it asked about.
  let typed = $state("");
  let whose = $state<GitOid | null>(null);

  interface Pages {
    readonly rows: readonly CommitAnswer[];
    readonly more: boolean;
    // What the newest page is: an older page never stands in for it,
    // because the newest one is what a person is waiting on.
    readonly newest: Answered<CommitsAnswer>;
    readonly edge: Seq | undefined;
  }

  // Every page folded into one reading: the newest page can grow
  // between asks and overlap the next, so an object id is shown once.
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
    const newest = readAnswer(answers.at(-1), (held) => ("commits" in held ? held.commits : undefined));
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

  // The page end is watched rather than clicked: a list that stops
  // forty rows in should not need a hand to say so.
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
    // The oldest row on the page diffs against the next page, so
    // opening it asks for one.
    if (last) grow();
  }

  function holds(oid: string): boolean {
    return pages.rows.some((row) => row.oid === oid);
  }

  // A parent the page holds is opened where it stands and brought into
  // view, so the press lands the reader on the row it named.
  function openRow(oid: string): void {
    open = oid;
    requestAnimationFrame(() => {
      document.getElementById(`commit-${oid}`)?.scrollIntoView({ block: "nearest" });
    });
  }

  const typedOid = $derived(Option.getOrNull(Schema.decodeOption(GitOid)(typed.trim())));
</script>

<div class="flex min-w-0 flex-col gap-base">
  <div class="flex min-w-0 flex-wrap items-center justify-between gap-x-wide gap-y-snug">
    <h2 class="text-note text-text-faint">{say($lang, "bld_commits")}</h2>
    <label class="flex min-w-0 items-center gap-snug text-note">
      <span class="shrink-0 whitespace-nowrap text-text-faint">{say($lang, "whose_label")}</span>
      <input
        class="h-control-sm w-[42ch] max-w-full min-w-0 rounded-control border border-edge-input bg-raised px-snug font-mono text-note placeholder:text-text-faint aria-invalid:border-alert"
        placeholder={say($lang, "whose_placeholder")}
        aria-invalid={typed.trim() !== "" && typedOid === null}
        bind:value={typed}
        onkeydown={(event) => {
          if (event.key === "Enter") whose = typedOid;
        }}
      />
    </label>
  </div>
  {#if typed.trim() !== "" && typedOid === null}
    <p class="text-note text-text-faint" role="status">{say($lang, "whose_not_oid")}</p>
  {/if}
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
