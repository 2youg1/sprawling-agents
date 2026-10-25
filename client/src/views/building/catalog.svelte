<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The building page's capabilities section (client/Spec.lean 4-50, its
  // tenth part): how each tool this building admits reaches its runs, and
  // the skills its reading room admits. The rows are the newest run's
  // tool table read against the `[catalog]` tables on disk
  // (`Query::Catalog`); a press edits one draft of one layer's table,
  // this building's or the city's, and one save sends that table whole.
  //
  // The card calls itself saved only once the city's answer holds what
  // it sent; a refusal is the city's toast and the draft stays as it
  // was, so saving again sends the same table.
  import { untrack } from "svelte";

  import { readAnswer } from "../../core/answered";
  import { configureCatalog, configureCityCatalog } from "../../core/commands";
  import { say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { ui } from "../../ui";
  import type { Address, Query } from "../../wire";
  import Unanswered from "../parts/unanswered.svelte";
  import { catalogLook, draftOf, sameTable } from "./catalog";
  import type { Scope, Table } from "./catalog";
  import Look from "./catalog.look.svelte";
  import Skills from "./skills.svelte";
  import type { Picked } from "./tree.svelte";

  interface Props {
    readonly building: Address;
    readonly onPick: (picked: Picked) => void;
  }

  const { building, onPick }: Props = $props();

  const u = ui();
  const { lang } = u;

  const question = $derived<Query>({ catalog: { building } });
  const asked = $derived(u.conn.asking.ask(question));
  const read = $derived(readAnswer($asked, (answer) => ("catalog" in answer ? answer.catalog : undefined)));
  const answer = $derived(read.kind === "held" ? read.value : undefined);

  let scope = $state<Scope>("building");
  let draft = $state.raw<Table>({ tools: {}, namespaces: {} });
  let edited = $state(false);
  // What the last save sent, until the city's answer holds it.
  let sent = $state.raw<Table | null>(null);
  let saved = $state(false);

  // The draft follows the city's answer until somebody edits it, and
  // follows it again once their edit has landed or the layer changes.
  $effect(() => {
    const now = answer;
    const layer = scope;
    if (now === undefined) return;
    const held = draftOf(now, layer);
    const waiting = untrack(() => sent);
    if (waiting !== null && sameTable(held, waiting)) {
      sent = null;
      edited = false;
      saved = true;
    }
    if (!untrack(() => edited)) draft = held;
  });

  function save(): void {
    const table = { tools: draft.tools, namespaces: draft.namespaces };
    const command = scope === "building" ? configureCatalog(building, table) : configureCityCatalog(table);
    if (u.send(command)) sent = draft;
  }

  const look = $derived(
    answer === undefined
      ? undefined
      : catalogLook({ answer, scope, draft, edited, now: u.now() }, $lang, {
          scope: (next) => {
            scope = next;
            edited = false;
            saved = false;
            sent = null;
            draft = draftOf(answer, next);
          },
          edit: (next) => {
            draft = next;
            edited = true;
            saved = false;
          },
          save,
        }),
  );
</script>

<section class="flex max-w-page min-w-0 flex-col gap-wide" aria-label={say($lang, "bld_catalog")}>
  <h2 class="sr-only">{say($lang, "bld_catalog")}</h2>
  {#if read.kind === "unavailable"}
    <Unanswered query={read.query} asked={question} />
  {:else if look === undefined}
    <p class="text-text-faint">…</p>
  {:else}
    <Look {...look} />
    {#if saved}
      <p class="text-note text-text-quiet" role="status">{say($lang, "catalog_saved")}</p>
    {/if}
  {/if}
  <div class="flex min-w-0 flex-col gap-snug border-t border-edge pt-wide">
    <p class="text-note text-text-faint">
      {say($lang, "catalog_skills_rules")}
      <a class="text-accent underline-offset-2 hover:underline" href={toFragment({ kind: "setup", group: "rules" })}>
        {say($lang, "catalog_skills_rules_link")}
      </a>
    </p>
    <Skills {building} {onPick} newest={answer?.run ?? undefined} />
  </div>
</section>
