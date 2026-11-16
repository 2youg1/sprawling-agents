<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts">
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
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Address, Answer, CatalogAnswer, Effort, Query } from "../../wire";
  import { choiceOf, tiersOf } from "./subagent";
  import type { Tier } from "./subagent";
  import { rulesAt } from "../settings/files";
  import { HELD, answered, awaitReceipt, refused, sent as markSent } from "../settings/saving";
  import type { Saving, Slot } from "../settings/saving";
  import { useBuildings } from "../shared/buildings";
  import { draftOf, selectionOf, settleSave } from "./catalog";
  import { catalogLook } from "./catalog_look";
  import type { Scope, Selection } from "./catalog";
  import type { CatalogPane } from "./catalog_look";
  import { takeCatalogFocus } from "./catalog_focus.svelte";
  import { lockOf, lockShare, withLock } from "./context_lock";
  import CatalogView from "./catalog_view.svelte";
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
  const rulesHeld = $derived(u.conn.asking.ask({ document: { at: rulesAt(building) } }));
  const rulesRead = $derived(readDocument($rulesHeld));
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
  let lockBox = $state("");
  let lockEdited = $state(false);
  let lockSaving = $state(false);
  $effect(() => {
    if (onDisk === null || lockEdited) return;
    const held = lockOf(onDisk);
    lockBox = held === null ? "" : String(held);
  });
  const lockField = $derived.by(
    (): { kind: "clear" } | { kind: "set"; tokens: number } | { kind: "invalid" } => {
      const trimmed = lockBox.trim();
      if (trimmed === "") return { kind: "clear" };
      const tokens = Number(trimmed);
      return Number.isInteger(tokens) && tokens > 0 ? { kind: "set", tokens } : { kind: "invalid" };
    },
  );
  const lockTokens = $derived(lockField.kind === "set" ? lockField.tokens : null);
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
    const tokens = lockField.kind === "set" ? lockField.tokens : null;
    const next = withLock(onDisk, tokens);
    if (!u.send(putRules(building, onDisk, next))) return;
    rulesDraft = withLock(rulesDraft, tokens);
    lockSaving = true;
    const mine = markSent(onDisk);
    rulesSaving = mine;
    awaitReceipt(mine, rulesSlot);
  }
  $effect(() => {
    if (!lockSaving) return;
    if (rulesSaving.kind === "saved") {
      lockEdited = false;
      lockSaving = false;
      return;
    }
    if (rulesSaving.kind === "refused" || rulesSaving.kind === "unverified") {
      lockSaving = false;
    }
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

<CatalogView
  {building}
  {read}
  {look}
  {pane}
  {saved}
  {question}
  {rulesFailed}
  {onDisk}
  {rulesDraft}
  {rulesSaving}
  {endpointRead}
  {endpointList}
  {providerModels}
  {levels}
  {shortcuts}
  {choiceBlocked}
  {provider}
  {modelId}
  {level}
  {lockBox}
  {lockEdited}
  {lockField}
  {modelWindow}
  {share}
  {saveRules}
  {saveLock}
  {saveChoice}
  {useTier}
  setRulesDraft={(next) => {
    rulesDraft = next;
  }}
  setRulesSaving={(next) => {
    rulesSaving = next;
  }}
  setProvider={(next) => {
    provider = next;
  }}
  setModelId={(next) => {
    modelId = next;
  }}
  setLevel={(next) => {
    level = next;
  }}
  setChoiceDirty={(next) => {
    choiceDirty = next;
  }}
  setLockBox={(next) => {
    lockBox = next;
  }}
  setLockEdited={(next) => {
    lockEdited = next;
  }}
/>
