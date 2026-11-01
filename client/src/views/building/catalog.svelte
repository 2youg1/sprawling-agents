<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The unified building catalog: one answer, one draft and one save.
  // A refusal leaves the draft untouched; only an answer equal to what
  // was sent marks it saved.
  import { untrack } from "svelte";
  import { readable } from "svelte/store";

  import { readAnswer } from "../../core/answered";
  import { configureCatalog, configureCityCatalog } from "../../core/commands";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Address, Answer, CatalogAnswer, Query } from "../../wire";
  import Unanswered from "../parts/unanswered.svelte";
  import { useBuildings } from "../shared/buildings";
  import { catalogLook, draftOf, sameSelection, sameTable, selectionOf } from "./catalog";
  import type { Scope, Selection } from "./catalog";
  import Look from "./catalog.look.svelte";

  interface Props {
    readonly building: Address;
  }

  const { building }: Props = $props();
  const u = ui();
  const { lang } = u;
  const buildings = useBuildings();

  const question = $derived<Query>({ catalog: { building } });
  const asked = $derived(u.conn.asking.ask(question));
  const read = $derived(
    readAnswer($asked, (answer) => ("catalog" in answer ? answer.catalog : undefined)),
  );
  const answer = $derived(read.kind === "held" ? read.value : undefined);

  let scope = $state<Scope>("building");
  let draft = $state.raw<Selection>({
    catalog: { tools: {}, namespaces: {}, skills: {} },
    references: [],
    skillPins: {},
    readingRoom: [],
  });
  let edited = $state(false);
  let sent = $state.raw<Selection | null>(null);
  let saved = $state(false);
  let unusedOnly = $state(false);
  let copyFrom = $state<Address | null>(null);

  function initial(answer: CatalogAnswer, layer: Scope): Selection {
    const selection = selectionOf(answer);
    return layer === "city" ? { ...selection, catalog: draftOf(answer, "city") } : selection;
  }

  $effect(() => {
    const now = answer;
    const layer = scope;
    if (now === undefined) return;
    const held = initial(now, layer);
    const waiting = untrack(() => sent);
    if (waiting !== null) {
      const landed =
        layer === "building"
          ? sameSelection(held, waiting)
          : sameTable(held.catalog, waiting.catalog);
      if (landed) {
        sent = null;
        edited = false;
        saved = true;
      }
    }
    if (!untrack(() => edited)) draft = held;
  });

  const copyAsked = $derived(
    copyFrom === null
      ? readable<Answer | undefined>(undefined)
      : u.conn.asking.ask({ catalog: { building: copyFrom } }),
  );
  const copied = $derived(
    readAnswer($copyAsked, (held) => ("catalog" in held ? held.catalog : undefined)),
  );

  $effect(() => {
    if (copyFrom === null || copied.kind !== "held") return;
    draft = selectionOf(copied.value);
    edited = true;
    saved = false;
    sent = null;
    copyFrom = null;
  });

  function save(): void {
    const command =
      scope === "building"
        ? configureCatalog(building, {
            catalog: draft.catalog,
            mcp_references: draft.references,
            skill_pins: draft.skillPins,
            reading_room: draft.readingRoom,
          })
        : configureCityCatalog(draft.catalog);
    if (u.send(command)) sent = draft;
  }

  const look = $derived(
    answer === undefined
      ? undefined
      : catalogLook(
          {
            answer,
            scope,
            draft,
            edited,
            now: u.now(),
            unusedOnly,
            buildings: $buildings,
          },
          $lang,
          {
            scope: (next) => {
              scope = next;
              edited = false;
              saved = false;
              sent = null;
              draft = initial(answer, next);
            },
            edit: (next) => {
              draft = next;
              edited = true;
              saved = false;
            },
            save,
            unused: (only) => {
              unusedOnly = only;
            },
            copy: (source) => {
              copyFrom = source;
            },
          },
        ),
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
</section>
