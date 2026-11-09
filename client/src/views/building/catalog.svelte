<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The unified building catalog: one answer, one draft and one save.
  // A refusal leaves the draft untouched; only an answer equal to what
  // was sent marks it saved. The tools / skills / MCP panes edit the
  // catalog table; the models and rules panes edit this building's
  // RULES.toml through the same PutRules door the settings page uses.
  import { untrack } from "svelte";
  import { readable } from "svelte/store";

  import { readAnswer } from "../../core/answered";
  import { configureCatalog, configureCityCatalog, putRules } from "../../core/commands";
  import { readDocument } from "../../core/document";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Address, Answer, CatalogAnswer, Query } from "../../wire";
  import Button from "../parts/button.svelte";
  import Unanswered from "../parts/unanswered.svelte";
  import { rulesAt } from "../settings/files";
  import {
    HELD,
    answered,
    awaitReceipt,
    edited as markEdited,
    refused,
    sent as markSent,
  } from "../settings/saving";
  import type { Saving, Slot } from "../settings/saving";
  import { useBuildings } from "../shared/buildings";
  import { catalogLook, draftOf, selectionOf, settleSave } from "./catalog";
  import type { CatalogPane, Scope, Selection } from "./catalog";
  import { takeCatalogFocus } from "./catalog_focus.svelte";
  import Look from "./catalog.look.svelte";
  import { textBoxWire } from "./text_box";
  import TextBox from "./text_box.look.svelte";

  interface Props {
    readonly building: Address;
  }

  const { building }: Props = $props();
  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const buildings = useBuildings();

  const question = $derived<Query>({ catalog: { building } });
  const asked = $derived(u.conn.asking.ask(question));
  const read = $derived(
    readAnswer($asked, (answer) => ("catalog" in answer ? answer.catalog : undefined)),
  );
  const answer = $derived(read.kind === "held" ? read.value : undefined);

  let scope = $state<Scope>("building");
  let pane = $state<CatalogPane>("tools");
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

  // A door under the composer may have asked for a pane before this
  // seat mounted; take it once.
  $effect(() => {
    const focus = takeCatalogFocus();
    switch (focus) {
      case null:
      case "worktree":
        return;
      case "mcp":
      case "models":
      case "rules":
      case "skills":
      case "tools":
        pane = focus;
    }
  });

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
      const settled = settleSave(
        held,
        untrack(() => draft),
        waiting,
        layer,
      );
      if (settled.landed) {
        sent = null;
        edited = settled.edited;
        saved = settled.saved;
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

  // RULES.toml for the models and rules panes: the same PutRules door
  // the settings page uses, so a change here is a change there.
  const rulesHeld = $derived(u.conn.asking.ask({ document: { at: rulesAt(building) } }));
  const onDisk = $derived.by(() => {
    const readRules = readDocument($rulesHeld);
    return readRules.kind === "held"
      ? readRules.value.text
      : readRules.kind === "unavailable"
        ? ""
        : null;
  });
  let rulesDraft = $state("");
  let rulesSaving = $state.raw<Saving>(HELD);
  const rulesSlot: Slot = { now: () => rulesSaving, mark: (next) => (rulesSaving = next) };
  let rulesWas: Address | null = null;
  $effect(() => {
    const at = building;
    if (onDisk === null) return;
    rulesSaving = answered(rulesSaving, onDisk);
    if (rulesWas !== at || rulesSaving.kind === "held" || rulesSaving.kind === "saved") {
      rulesDraft = onDisk;
    }
    if (rulesWas !== at) rulesSaving = HELD;
    rulesWas = at;
  });
  let seen = $belief.refusal;
  $effect(() => {
    const error = $belief.refusal;
    if (error === null || error === seen) return;
    seen = error;
    rulesSaving = refused(rulesSaving, error);
  });

  function saveRules(): void {
    if (onDisk === null || !u.send(putRules(building, onDisk, rulesDraft))) return;
    const mine = markSent(onDisk);
    rulesSaving = mine;
    awaitReceipt(mine, rulesSlot);
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
            pane,
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
            pane: (next) => {
              pane = next;
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

{#snippet rules()}
  <div class="flex min-w-0 flex-col gap-base">
    {#if onDisk === null}
      <p class="text-text-faint">…</p>
    {:else}
      <TextBox
        size="document"
        wire={textBoxWire(rulesDraft, say($lang, "rules_title"), false, (next) => {
          rulesDraft = next;
          rulesSaving = markEdited(next !== onDisk);
        })}
      />
      <span class="flex flex-wrap items-center gap-base">
        <Button
          label={say($lang, "catalog_save_rules")}
          tone="primary"
          {...rulesDraft === onDisk ? { why: say($lang, "catalog_unchanged") } : {}}
          onPress={saveRules}
        />
        {#if rulesSaving.kind === "saved"}
          <span class="text-note text-text-quiet" role="status">{say($lang, "catalog_saved")}</span>
        {/if}
      </span>
    {/if}
  </div>
{/snippet}

<section class="flex max-w-page min-w-0 flex-col gap-wide" aria-label={say($lang, "bld_catalog")}>
  <h2 class="sr-only">{say($lang, "bld_catalog")}</h2>
  {#if read.kind === "unavailable"}
    <Unanswered query={read.query} asked={question} />
  {:else if look === undefined}
    <p class="text-text-faint">…</p>
  {:else}
    <Look {...look} rules={pane === "models" || pane === "rules" ? rules : undefined} />
    {#if saved}
      <p class="text-note text-text-quiet" role="status">{say($lang, "catalog_saved")}</p>
    {/if}
  {/if}
</section>
