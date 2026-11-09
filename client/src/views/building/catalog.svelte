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
  // catalog table; the models, context and rules panes edit this
  // building's RULES.toml through the same PutRules door.
  import { untrack } from "svelte";
  import { readable } from "svelte/store";

  import { QUERIES } from "../../core/asking";
  import { readAnswer } from "../../core/answered";
  import {
    configureCatalog,
    configureCityCatalog,
    putRules,
    setWorkspaceChoice,
  } from "../../core/commands";
  import { readDocument } from "../../core/document";
  import { fill, say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { kilo } from "../../core/time";
  import { ui } from "../../ui";
  import type { Address, Answer, CatalogAnswer, Effort, Query } from "../../wire";
  import { TIERS, choiceOf, tiersOf } from "./subagent";
  import type { Tier } from "./subagent";
  import Button from "../parts/button.svelte";
  import Field from "../parts/field.svelte";
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
  import { lockOf, lockShare, withLock } from "./context_lock";
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
        return;
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
  const rulesRead = $derived(readDocument($rulesHeld));
  // Missing file is an empty base the writer may initialise. Any other
  // unanswered document read stays unanswered; it is not an empty file.
  const onDisk = $derived.by(() => {
    switch (rulesRead.kind) {
      case "held":
        return rulesRead.value.text;
      case "unavailable":
        return rulesRead.reason === null ? "" : null;
      case "asking":
        return null;
    }
  });
  const rulesFailed = $derived(
    rulesRead.kind === "unavailable" && rulesRead.reason !== null ? rulesRead : null,
  );
  let rulesDraft = $state("");
  let rulesSaving = $state.raw<Saving>(HELD);
  const rulesSlot: Slot = { now: () => rulesSaving, mark: (next) => (rulesSaving = next) };
  let rulesWas: Address | null = null;
  $effect(() => {
    const at = building;
    if (onDisk === null) return;
    const prior = untrack(() => rulesSaving);
    rulesSaving = answered(rulesSaving, onDisk);
    // A choice-only write must not wipe unsaved RULES.toml edits on the
    // same pane: sync the draft only when it still matches the base the
    // save was made against (or when the card is held / the building moved).
    if (rulesWas !== at || rulesSaving.kind === "held") {
      rulesDraft = onDisk;
    } else if (
      rulesSaving.kind === "saved" &&
      prior.kind === "saving" &&
      rulesDraft === prior.from
    ) {
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

  // Context lock field, kept beside the RULES draft so PutRules is the
  // only write door for this pane.
  let lockBox = $state("");
  let lockEdited = $state(false);
  $effect(() => {
    if (onDisk === null || lockEdited) return;
    const held = lockOf(onDisk);
    lockBox = held === null ? "" : String(held);
  });
  // Empty clears the key; a non-positive or non-integer value is not a
  // clear — save must refuse rather than delete the lock on disk.
  const lockField = $derived.by(
    (): { kind: "clear" } | { kind: "set"; tokens: number } | { kind: "invalid" } => {
      const trimmed = lockBox.trim();
      if (trimmed === "") return { kind: "clear" };
      const tokens = Number(trimmed);
      return Number.isInteger(tokens) && tokens > 0 ? { kind: "set", tokens } : { kind: "invalid" };
    },
  );
  const lockTokens = $derived(lockField.kind === "set" ? lockField.tokens : null);

  // The direct sub-agent model. Easy / medium / hard fill these three
  // selects; saving writes `[workspace_models.choice]` and leaves the
  // shortcuts in the file. The same endpoints answer feeds the context
  // pane's model-window share.
  const endpointAsked = $derived(u.conn.asking.ask(QUERIES.endpoints));
  const endpointRead = $derived(
    readAnswer($endpointAsked, (held) => ("endpoints" in held ? held.endpoints : undefined)),
  );
  const endpointList = $derived(endpointRead.kind === "held" ? endpointRead.value.endpoints : []);
  const modelWindow = $derived.by(() => {
    let largest: number | null = null;
    for (const endpoint of endpointList) {
      for (const model of endpoint.models) {
        const window = model.context_tokens;
        if (window === undefined || window === null || window <= 0) continue;
        if (largest === null || window > largest) largest = window;
      }
    }
    return largest;
  });
  const share = $derived(
    lockTokens === null || modelWindow === null ? null : lockShare(lockTokens, modelWindow),
  );

  function saveLock(): void {
    if (onDisk === null || lockField.kind === "invalid" || rulesSaving.kind === "saving") return;
    // PutRules writes only the disk base plus the lock key: unfinished
    // rules-pane prose is not committed by a lock save. The editor draft
    // still receives the same key rewrite so unsaved prose is not wiped.
    const tokens = lockField.kind === "set" ? lockField.tokens : null;
    const next = withLock(onDisk, tokens);
    if (!u.send(putRules(building, onDisk, next))) return;
    rulesDraft = withLock(rulesDraft, tokens);
    // Keep lockEdited until the write answers: clearing it here re-runs
    // the sync effect against the still-old onDisk and wipes the box.
    const mine = markSent(onDisk);
    rulesSaving = mine;
    awaitReceipt(mine, rulesSlot);
  }
  $effect(() => {
    if (rulesSaving.kind === "saved") lockEdited = false;
  });

  let provider = $state("");
  let modelId = $state("");
  let level = $state<Effort>("medium");
  let choiceDirty = $state(false);
  let choiceAt: Address | null = null;
  function applyChoice(text: string): void {
    const picked = choiceOf(text) ?? tiersOf(text).medium;
    if (picked === undefined) {
      provider = "";
      modelId = "";
      level = "medium";
      return;
    }
    provider = picked.endpoint;
    modelId = picked.model;
    level = picked.effort;
  }
  $effect(() => {
    const at = building;
    const text = onDisk;
    if (text === null) return;
    const moved = choiceAt !== at;
    choiceAt = at;
    if (moved) {
      applyChoice(text);
      choiceDirty = false;
      return;
    }
    if (choiceDirty) return;
    applyChoice(text);
  });
  $effect(() => {
    if (rulesSaving.kind === "saved") choiceDirty = false;
  });
  const providerModels = $derived(
    endpointList.find((each) => each.name === provider)?.models ?? [],
  );
  const levels = $derived.by((): readonly Effort[] => {
    return providerModels.find((each) => each.id === modelId)?.thinking.levels ?? [];
  });
  const shortcuts = $derived(onDisk === null ? {} : tiersOf(onDisk));
  const choiceBlocked = $derived.by((): string | undefined => {
    if (rulesSaving.kind === "saving") return say($lang, "subagent_saving");
    if (provider === "" || modelId === "") return say($lang, "subagent_no_endpoint");
    if (levels.length === 0 || !levels.includes(level)) return say($lang, "subagent_no_effort");
    return undefined;
  });

  function useTier(tier: Tier): void {
    const picked = shortcuts[tier];
    if (picked === undefined) return;
    provider = picked.endpoint;
    modelId = picked.model;
    level = picked.effort;
    choiceDirty = true;
  }

  function saveChoice(): void {
    if (onDisk === null || choiceBlocked !== undefined || rulesSaving.kind === "saving") return;
    if (!u.send(setWorkspaceChoice(building, onDisk, provider, modelId, level))) return;
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
    {#if rulesFailed !== null}
      <Unanswered
        query={rulesFailed.query}
        reason={rulesFailed.reason}
        asked={{ document: { at: rulesAt(building) } }}
      />
    {:else if onDisk === null}
      <p class="text-text-faint">…</p>
    {:else}
      {#if pane === "models"}
        {#if endpointRead.kind === "unavailable"}
          <Unanswered
            query={endpointRead.query}
            reason={endpointRead.reason}
            asked={QUERIES.endpoints}
          />
        {:else if endpointRead.kind === "asking"}
          <p class="text-text-faint">…</p>
        {:else if endpointList.length === 0}
          <p class="text-note text-text-faint">{say($lang, "subagent_no_endpoint")}</p>
        {:else}
          <div class="flex min-w-0 flex-wrap items-end gap-base">
            <label class="flex min-w-0 flex-col gap-tight text-note">
              <span class="text-text-quiet">{say($lang, "subagent_provider")}</span>
              <select
                class="h-control max-w-full rounded-control border border-edge-input bg-raised px-snug text-note text-text"
                value={provider}
                onchange={(event) => {
                  provider = event.currentTarget.value;
                  const next = endpointList.find((each) => each.name === provider);
                  modelId = next?.models[0]?.id ?? "";
                  const offered = next?.models[0]?.thinking.levels ?? [];
                  const first = offered[0];
                  level = first ?? "medium";
                  choiceDirty = true;
                }}
              >
                <option value=""></option>
                {#each endpointList as endpoint (endpoint.name)}
                  <option value={endpoint.name}>{endpoint.label}</option>
                {/each}
              </select>
            </label>
            <label class="flex min-w-0 flex-col gap-tight text-note">
              <span class="text-text-quiet">{say($lang, "subagent_model")}</span>
              <select
                class="h-control max-w-full rounded-control border border-edge-input bg-raised px-snug text-note text-text"
                value={modelId}
                onchange={(event) => {
                  modelId = event.currentTarget.value;
                  const offered =
                    providerModels.find((each) => each.id === modelId)?.thinking.levels ?? [];
                  const first = offered[0];
                  if (first === undefined) level = "medium";
                  else if (!offered.includes(level)) level = first;
                  choiceDirty = true;
                }}
              >
                {#each providerModels as facts (facts.id)}
                  <option value={facts.id}>{facts.id}</option>
                {/each}
              </select>
            </label>
            <label class="flex min-w-0 flex-col gap-tight text-note">
              <span class="text-text-quiet">{say($lang, "subagent_effort")}</span>
              <select
                class="h-control max-w-full rounded-control border border-edge-input bg-raised px-snug text-note text-text"
                value={level}
                onchange={(event) => {
                  const next = levels.find((each) => each === event.currentTarget.value);
                  if (next !== undefined) {
                    level = next;
                    choiceDirty = true;
                  }
                }}
              >
                {#each levels as effort (effort)}
                  <option value={effort}>{effort}</option>
                {/each}
              </select>
            </label>
          </div>
          {#if modelId !== "" && levels.length === 0}
            <p class="text-note text-text-faint">{say($lang, "subagent_no_effort")}</p>
          {/if}
          <span class="flex flex-wrap items-center gap-base">
            {#each TIERS as tier (tier)}
              {#if shortcuts[tier] !== undefined}
                <Button
                  label={fill(say($lang, "subagent_shortcut"), { tier })}
                  tone="quiet"
                  onPress={() => {
                    useTier(tier);
                  }}
                />
              {/if}
            {/each}
            <Button
              label={say($lang, "subagent_save")}
              tone="primary"
              loading={rulesSaving.kind === "saving"}
              {...choiceBlocked === undefined ? {} : { why: choiceBlocked }}
              onPress={saveChoice}
            />
            {#if rulesSaving.kind === "saved"}
              <span class="text-note text-text-quiet" role="status"
                >{say($lang, "catalog_saved")}</span
              >
            {:else if rulesSaving.kind === "refused"}
              <span class="text-note text-text-quiet" role="status"
                >{rulesSaving.error.recovery}</span
              >
            {/if}
          </span>
        {/if}
      {/if}
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
        {:else if rulesSaving.kind === "refused"}
          <span class="text-note text-text-quiet" role="status">{rulesSaving.error.recovery}</span>
        {/if}
      </span>
    {/if}
  </div>
{/snippet}

{#snippet context()}
  <div class="flex min-w-0 flex-col gap-base">
    <Field
      label={say($lang, "catalog_context_lock")}
      kind="number"
      step={1}
      suffix={say($lang, "catalog_context_lock_unit")}
      mono
      placeholder={say($lang, "catalog_context_lock_hint")}
      value={lockBox}
      onInput={(next: string) => {
        lockBox = next;
        lockEdited = true;
      }}
    />
    {#if modelWindow === null}
      <p class="text-note text-text-faint">{say($lang, "catalog_context_window_unknown")}</p>
    {:else}
      <p class="text-note text-text-quiet">
        {fill(say($lang, "catalog_context_window"), { window: kilo(modelWindow) })}
        {#if share !== null}
          · {fill(say($lang, "catalog_context_share"), { n: String(share) })}
        {/if}
      </p>
    {/if}
    <p class="text-note text-text-faint">
      {say($lang, "catalog_context_warm")}
      <a
        class="text-accent underline-offset-2 hover:underline"
        href={toFragment({ kind: "setup", group: "accounts" })}
      >
        {say($lang, "nav_settings")}
      </a>
    </p>
    <span class="flex flex-wrap items-center gap-base">
      <Button
        label={say($lang, "catalog_context_save")}
        tone="primary"
        loading={rulesSaving.kind === "saving"}
        {...lockEdited && lockField.kind !== "invalid" && rulesSaving.kind !== "saving"
          ? {}
          : {
              why: say(
                $lang,
                lockField.kind === "invalid"
                  ? "catalog_context_lock_invalid"
                  : rulesSaving.kind === "saving"
                    ? "subagent_saving"
                    : "catalog_unchanged",
              ),
            }}
        onPress={saveLock}
      />
      {#if rulesSaving.kind === "saved"}
        <span class="text-note text-text-quiet" role="status">{say($lang, "catalog_saved")}</span>
      {:else if rulesSaving.kind === "refused"}
        <span class="text-note text-text-quiet" role="status">{rulesSaving.error.recovery}</span>
      {/if}
    </span>
  </div>
{/snippet}

<section class="flex max-w-page min-w-0 flex-col gap-wide" aria-label={say($lang, "bld_catalog")}>
  <h2 class="sr-only">{say($lang, "bld_catalog")}</h2>
  {#if read.kind === "unavailable"}
    <Unanswered query={read.query} asked={question} />
  {:else if look === undefined}
    <p class="text-text-faint">…</p>
  {:else}
    <Look
      {...look}
      rules={pane === "models" || pane === "rules"
        ? rules
        : pane === "context"
          ? context
          : undefined}
    />
    {#if saved}
      <p class="text-note text-text-quiet" role="status">{say($lang, "catalog_saved")}</p>
    {/if}
  {/if}
</section>
