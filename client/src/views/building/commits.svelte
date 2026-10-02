<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The commits a building made, newest first, and the way back from a
  // line of code to the session that wrote it. A row names the resident
  // on the left and the session on the right; the session is a link to
  // that room's conversation and nothing more (D52: from a diff to a
  // session is a jump, not a new verb). Opening a row lists the files
  // it changed against the commit before it; opening a file shows the
  // patch. Scrolling to the end asks for the next page.
  //
  // The row keeps one secondary action behind the row itself: copying
  // the commit id is how a person carries this line into a review, and
  // it arrives with its receipt (ux A7).

  import type { Attachment } from "svelte/attachments";
  import { SvelteSet } from "svelte/reactivity";
  import { derived } from "svelte/store";

  import { readAnswer } from "../../core/answered";
  import type { Answered } from "../../core/answered";
  import { commitsQuery } from "../../core/asking";
  import { fill, say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { clock, usd } from "../../core/time";
  import { ui } from "../../ui";
  import type { Address, Answer, CommitAnswer, CommitsAnswer, Effort, Seq } from "../../wire";
  import Changes from "../changes.svelte";
  import { shortOid } from "../changes";
  import Glyph from "../parts/glyph.svelte";
  import { RowList } from "../parts/row.svelte";
  import Tip from "../parts/tip.svelte";
  import Unanswered from "../parts/unanswered.svelte";

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
  // Whose copy receipt is showing right now (ux A7).
  let receipt = $state<string | null>(null);

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
    return {
      rows,
      more: tail?.more === true,
      newest,
      edge: tail?.commits.at(-1)?.seq,
    };
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

  // A copy receipt lasts long enough to be seen and no longer (ux A7).
  $effect(() => {
    const at = receipt;
    if (at === null) return;
    const timer = setTimeout(() => {
      if (receipt === at) receipt = null;
    }, 1200);
    return () => {
      clearTimeout(timer);
    };
  });

  function copy(oid: string): void {
    void navigator.clipboard.writeText(oid);
    receipt = oid;
  }

  function toggle(commit: CommitAnswer, last: boolean): void {
    open = open === commit.oid ? null : commit.oid;
    // The oldest row on the page diffs against the next page, so
    // opening it asks for one.
    if (last) grow();
  }

  function effortTail(effort: Effort | null | undefined): string {
    return effort === null || effort === undefined ? "" : ` · ${effort}`;
  }
</script>

{#snippet lineage(commit: CommitAnswer)}
  {#if commit.lineage.length > 1}
    <p class="flex flex-wrap items-baseline gap-snug pb-snug text-note text-text-faint">
      <span>{say($lang, "commits_lineage")}</span>
      {#each commit.lineage.slice(1) as run (run)}
        <a
          href={toFragment({ kind: "run", run })}
          class="font-mono text-text-faint hover:text-text-quiet">{run.slice(0, 8)}</a
        >
      {/each}
    </p>
  {/if}
{/snippet}

{#snippet row(commit: CommitAnswer, older: CommitAnswer | null, last: boolean)}
  {const isOpen = open === commit.oid}
  <li class="group border-b border-edge">
    <div class="flex items-center gap-base py-snug text-note">
      <button
        type="button"
        class="flex min-w-0 flex-1 items-center gap-base text-left hover:text-text"
        aria-expanded={isOpen}
        onclick={() => {
          toggle(commit, last);
        }}
      >
        <Glyph
          name="chevron"
          size="sm"
          class="shrink-0 text-text-faint transition-transform {isOpen ? 'rotate-90' : ''}"
        />
        <span class="truncate font-mono text-text-quiet">{commit.actor}</span>
        <span class="shrink-0 font-mono text-text-faint">{shortOid(commit.oid)}</span>
        {#if commit.model !== ""}
          <span class="hidden shrink-0 truncate text-text-faint md:inline"
            >{commit.model}{effortTail(commit.effort)}</span
          >
        {/if}
        {#if commit.spent > 0}
          <span class="hidden shrink-0 text-text-faint md:inline"
            >{fill(say($lang, "commits_spent"), { usd: usd(commit.spent) })}</span
          >
        {/if}
        {#if commit.lineage.length > 1}
          <Tip text={say($lang, "commits_lineage")}>
            {#snippet children(hint)}
              <span
                class="shrink-0 rounded-pill bg-raised px-snug text-text-faint"
                aria-describedby={hint}
              >
                {fill(say($lang, "commits_succeeded"), {
                  n: String(commit.lineage.length - 1),
                })}
              </span>
            {/snippet}
          </Tip>
        {/if}
      </button>
      <Tip text={say($lang, "tree_transcript")}>
        {#snippet children(hint)}
          <a
            href={toFragment({ kind: "run", run: commit.run })}
            class="shrink-0 whitespace-nowrap text-text-faint hover:text-text-quiet"
            aria-describedby={hint}>{clock($lang, commit.at)}</a
          >
        {/snippet}
      </Tip>
      {#if commit.session !== null && commit.session !== undefined}
        <a
          href={toFragment({ kind: "talk", address: commit.actor })}
          class="inline-flex h-control-sm shrink-0 items-center rounded-control bg-raised px-snug text-label text-text-quiet hover:bg-raised-hover hover:text-text"
          >{commit.session} →</a
        >
      {/if}
      <!-- The secondary action appears with the row rather than in it
           (ux A7): it must not compete with the row's own way in, and a
           focus inside the row reveals it the same way a pointer does. -->
      <span
        class="flex shrink-0 items-center opacity-0 transition-opacity ease-leave group-hover:opacity-100 group-hover:ease-arrive group-focus-within:opacity-100 group-focus-within:ease-arrive"
      >
        <button
          type="button"
          class="inline-flex h-control-sm items-center gap-tight rounded-control bg-raised px-snug text-label text-text-quiet hover:bg-raised-hover hover:text-text"
          onclick={() => {
            copy(commit.oid);
          }}
        >
          {#if receipt === commit.oid}
            <Glyph name="check" size="sm" />
            {say($lang, "run_prompt_copied")}
          {:else}
            {say($lang, "run_prompt_copy")}
          {/if}
        </button>
      </span>
    </div>
    {#if isOpen}
      <div class="pb-base pl-wide">
        <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
        {@render lineage(commit)}
        {#if older !== null}
          <Changes base={older.oid} head={commit.oid} talk={building} />
        {:else}
          <p class="text-text-faint">{last ? say($lang, "commits_first") : "…"}</p>
        {/if}
      </div>
    {/if}
  </li>
{/snippet}

{#snippet rows()}
  {#each pages.rows as commit, index (commit.oid)}
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
    {@render row(commit, pages.rows[index + 1] ?? null, !pages.more && index === pages.rows.length - 1)}
  {/each}
  <li {@attach sentinel} class="h-hair" aria-hidden="true"></li>
{/snippet}

<div>
  <h2 class="mb-base text-heading font-heading">{say($lang, "bld_commits")}</h2>
  {#if pages.rows.length > 0}
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression, @typescript-eslint/no-unsafe-call (a snippet call is the render itself; the typechecker types a snippet exported from a .svelte module as unresolvable) -->
    {@render RowList({ label: say($lang, "bld_commits"), rows })}
  {:else if pages.newest.kind === "held"}
    <p class="text-text-faint">{say($lang, "commits_empty")}</p>
  {:else if pages.newest.kind === "asking"}
    <p class="text-text-faint">…</p>
  {/if}
  {#if pages.newest.kind === "unavailable"}
    <Unanswered query={pages.newest.query} asked={commitsQuery(building, befores.at(-1) ?? null)} />
  {/if}
  {#if pages.more}
    <p class="py-base text-center text-text-faint">…</p>
  {/if}
</div>
