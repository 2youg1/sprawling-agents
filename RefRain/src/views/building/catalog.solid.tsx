// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, untrack } from "solid-js";
import { readAnswer } from "../../core/answered";
import { QUERIES } from "../../core/asking";
import {
  configureCatalog,
  configureCityCatalog,
  putRules,
  setWorkspaceChoice,
} from "../../core/commands";
import { readDocument } from "../../core/document";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Address, CatalogAnswer, Effort, Query } from "../../wire";
import { rulesAt } from "../settings/files";
import {
  HELD,
  answered,
  awaitReceipt,
  refused,
  sent as markSent,
  type Saving,
  type Slot,
} from "../settings/saving";
import { useBuildings } from "../shared/buildings";
import { draftOf, selectionOf, settleSave, type Scope, type Selection } from "./catalog";
import { catalogFocus, takeCatalogFocus } from "./catalog_focus";
import { catalogLook, type CatalogPane } from "./catalog_look";
import CatalogView from "./catalog_view.solid";
import { lockOf, lockShare, withLock } from "./context_lock";
import { choiceOf, tiersOf, type Tier } from "./subagent";

const EMPTY: Selection = {
  catalog: { tools: {}, namespaces: {}, skills: {} },
  references: [],
  skillPins: {},
  readingRoom: [],
};

function initial(answer: CatalogAnswer, layer: Scope): Selection {
  const selection = selectionOf(answer);
  return layer === "city" ? { ...selection, catalog: draftOf(answer, layer) } : selection;
}

type Lock = { kind: "clear" } | { kind: "set"; tokens: number } | { kind: "invalid" };

export default function Catalog(props: { readonly building: Address }) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const buildings = storeValue(useBuildings());
  const focus = storeValue(catalogFocus);
  const question = createMemo<Query>(() => ({ catalog: { building: props.building } }));
  const asked = storeValue(() => u.conn.asking.ask(question()));
  const read = createMemo(() =>
    readAnswer(asked(), (answer) => ("catalog" in answer ? answer.catalog : undefined)),
  );
  const answer = createMemo(() => {
    const held = read();
    return held.kind === "held" ? held.value : undefined;
  });

  const [scope, setScope] = createSignal<Scope>("building");
  const [pane, setPane] = createSignal<CatalogPane>("tools");
  const [draft, setDraft] = createSignal<Selection>(EMPTY);
  const [edited, setEdited] = createSignal(false);
  const [sent, setSent] = createSignal<Selection | null>(null);
  const [saved, setSaved] = createSignal(false);
  const [unusedOnly, setUnusedOnly] = createSignal(false);
  const [copyFrom, setCopyFrom] = createSignal<Address | null>(null);

  createEffect(focus, (wanted) => {
    if (wanted === null) return;
    const next = takeCatalogFocus();
    if (next !== null && next !== "worktree") setPane(next);
  });
  createEffect(
    () => [answer(), scope()] as const,
    ([now, layer]) => {
      if (now === undefined) return;
      const held = initial(now, layer);
      const waiting = sent();
      let retainDraft = edited();
      if (waiting !== null) {
        const settled = settleSave(held, draft(), waiting, layer);
        if (settled.landed) {
          setSent(null);
          setEdited(settled.edited);
          setSaved(settled.saved);
          retainDraft = settled.edited;
        }
      }
      if (!retainDraft) setDraft(held);
    },
  );

  const copyAsked = storeValue(() => {
    const from = copyFrom();
    return from === null ? undefined : u.conn.asking.ask({ catalog: { building: from } });
  });
  createEffect(copyAsked, (reply) => {
    if (copyFrom() === null || reply === undefined || !("catalog" in reply)) return;
    setDraft(selectionOf(reply.catalog));
    setEdited(true);
    setSaved(false);
    setSent(null);
    setCopyFrom(null);
  });

  function save(): void {
    const selection = draft();
    const command =
      scope() === "building"
        ? configureCatalog(props.building, {
            catalog: selection.catalog,
            mcp_references: selection.references,
            skill_pins: selection.skillPins,
            reading_room: selection.readingRoom,
          })
        : configureCityCatalog(selection.catalog);
    if (u.send(command)) setSent(selection);
  }

  const rulesHeld = storeValue(() =>
    u.conn.asking.ask({ document: { at: rulesAt(props.building) } }),
  );
  const rulesRead = createMemo(() => readDocument(rulesHeld()));
  const onDisk = createMemo(() => {
    const read = rulesRead();
    switch (read.kind) {
      case "held":
        return read.value.text;
      case "unavailable":
        return read.reason === null ? "" : null;
      case "asking":
        return null;
    }
  });
  const rulesFailed = () => {
    const read = rulesRead();
    return read.kind === "unavailable" && read.reason !== null ? read : null;
  };
  const [rulesDraft, setRulesDraft] = createSignal("");
  const [rulesSaving, setRulesSaving] = createSignal<Saving>(HELD);
  const rulesSlot: Slot = {
    now: rulesSaving,
    mark: (next) => {
      setRulesSaving(next);
    },
  };
  let rulesWas: Address | null = null;
  createEffect(
    () => [props.building, onDisk()] as const,
    ([at, text]) => {
      if (text === null) return;
      const prior = rulesSaving();
      const next = answered(prior, text);
      if (
        rulesWas !== at ||
        next.kind === "held" ||
        (next.kind === "saved" && prior.kind === "saving" && rulesDraft() === prior.from)
      ) {
        setRulesDraft(text);
      }
      setRulesSaving(rulesWas === at ? next : HELD);
      rulesWas = at;
    },
  );
  let seen = untrack(() => belief().refusal);
  createEffect(
    () => belief().refusal,
    (error) => {
      if (error === null || error === seen) return;
      seen = error;
      setRulesSaving((held) => refused(held, error));
    },
  );

  function awaiting(text: string): void {
    const mine = markSent(text);
    setRulesSaving(mine);
    awaitReceipt(mine, rulesSlot);
  }
  function saveRules(): void {
    const text = onDisk();
    if (text !== null && u.send(putRules(props.building, text, rulesDraft()))) awaiting(text);
  }

  const [lockBox, setLockBox] = createSignal("");
  const [lockEdited, setLockEdited] = createSignal(false);
  const [lockSaving, setLockSaving] = createSignal(false);
  createEffect(
    () => [onDisk(), lockEdited()] as const,
    ([text, edited]) => {
      if (text === null || edited) return;
      const held = lockOf(text);
      setLockBox(held === null ? "" : String(held));
    },
  );
  const lockField = createMemo((): Lock => {
    const trimmed = lockBox().trim();
    if (trimmed === "") return { kind: "clear" };
    const tokens = Number(trimmed);
    return Number.isInteger(tokens) && tokens > 0 ? { kind: "set", tokens } : { kind: "invalid" };
  });

  const endpointAsked = storeValue(u.conn.asking.ask(QUERIES.endpoints));
  const endpointRead = createMemo(() =>
    readAnswer(endpointAsked(), (held) => ("endpoints" in held ? held.endpoints : undefined)),
  );
  const endpointList = createMemo(() => {
    const held = endpointRead();
    return held.kind === "held" ? held.value.endpoints : [];
  });
  const modelWindow = createMemo(() => {
    let largest: number | null = null;
    for (const endpoint of endpointList()) {
      for (const model of endpoint.models) {
        const window = model.context_tokens;
        if (window == null || window <= 0) continue;
        if (largest === null || window > largest) largest = window;
      }
    }
    return largest;
  });
  const share = () => {
    const locked = lockField();
    const window = modelWindow();
    return locked.kind === "set" && window !== null ? lockShare(locked.tokens, window) : null;
  };
  function saveLock(): void {
    const text = onDisk();
    const field = lockField();
    if (text === null || field.kind === "invalid" || rulesSaving().kind === "saving") return;
    const tokens = field.kind === "set" ? field.tokens : null;
    if (!u.send(putRules(props.building, text, withLock(text, tokens)))) return;
    setRulesDraft((held) => withLock(held, tokens));
    setLockSaving(true);
    awaiting(text);
  }
  createEffect(rulesSaving, (receipt) => {
    if (!lockSaving()) return;
    if (receipt.kind === "saved") setLockEdited(false);
    if (receipt.kind === "saved" || receipt.kind === "refused" || receipt.kind === "unverified")
      setLockSaving(false);
  });

  const [provider, setProvider] = createSignal("");
  const [modelId, setModelId] = createSignal("");
  const [level, setLevel] = createSignal<Effort>("medium");
  const [choiceDirty, setChoiceDirty] = createSignal(false);
  let choiceAt: Address | null = null;
  function applyChoice(text: string): void {
    const picked = choiceOf(text) ?? tiersOf(text).medium;
    setProvider(picked?.endpoint ?? "");
    setModelId(picked?.model ?? "");
    setLevel(picked?.effort ?? "medium");
  }
  createEffect(
    () => [props.building, onDisk(), choiceDirty()] as const,
    ([at, text, dirty]) => {
      if (text === null) return;
      const moved = choiceAt !== at;
      choiceAt = at;
      if (moved) {
        applyChoice(text);
        setChoiceDirty(false);
      } else if (!dirty) {
        applyChoice(text);
      }
    },
  );
  createEffect(rulesSaving, (receipt) => {
    if (receipt.kind === "saved") setChoiceDirty(false);
  });
  const providerModels = () =>
    endpointList().find((endpoint) => endpoint.name === provider())?.models ?? [];
  const levels = (): readonly Effort[] =>
    providerModels().find((model) => model.id === modelId())?.thinking.levels ?? [];
  const shortcuts = () => {
    const text = onDisk();
    return text === null ? {} : tiersOf(text);
  };
  const choiceBlocked = (): string | undefined => {
    if (rulesSaving().kind === "saving") return say(lang(), "subagent_saving");
    if (provider() === "" || modelId() === "") return say(lang(), "subagent_no_endpoint");
    return levels().includes(level()) ? undefined : say(lang(), "subagent_no_effort");
  };
  function useTier(tier: Tier): void {
    const picked = shortcuts()[tier];
    if (picked === undefined) return;
    setProvider(picked.endpoint);
    setModelId(picked.model);
    setLevel(picked.effort);
    setChoiceDirty(true);
  }
  function saveChoice(): void {
    const text = onDisk();
    if (text === null || choiceBlocked() !== undefined) return;
    if (u.send(setWorkspaceChoice(props.building, text, provider(), modelId(), level())))
      awaiting(text);
  }

  const look = createMemo(() => {
    const held = answer();
    if (held === undefined) return undefined;
    return catalogLook(
      {
        answer: held,
        scope: scope(),
        draft: draft(),
        edited: edited(),
        now: u.now(),
        unusedOnly: unusedOnly(),
        buildings: buildings(),
        pane: pane(),
      },
      lang(),
      {
        scope: (next) => {
          setScope(next);
          setEdited(false);
          setSaved(false);
          setSent(null);
          setDraft(initial(held, next));
        },
        pane: (next) => {
          setPane(next);
        },
        edit: (next) => {
          setDraft(next);
          setEdited(true);
          setSaved(false);
        },
        save,
        unused: (only) => {
          setUnusedOnly(only);
        },
        copy: (source) => {
          setCopyFrom(source);
        },
      },
    );
  });

  return (
    <CatalogView
      building={props.building}
      read={read()}
      look={look()}
      pane={pane()}
      saved={saved()}
      question={question()}
      rulesFailed={rulesFailed()}
      onDisk={onDisk()}
      rulesDraft={rulesDraft()}
      rulesSaving={rulesSaving()}
      endpointRead={endpointRead()}
      endpointList={endpointList()}
      providerModels={providerModels()}
      levels={levels()}
      shortcuts={shortcuts()}
      choiceBlocked={choiceBlocked()}
      provider={provider()}
      modelId={modelId()}
      level={level()}
      lockBox={lockBox()}
      lockEdited={lockEdited()}
      lockField={lockField()}
      modelWindow={modelWindow()}
      share={share()}
      saveRules={saveRules}
      saveLock={saveLock}
      saveChoice={saveChoice}
      useTier={useTier}
      setRulesDraft={setRulesDraft}
      setRulesSaving={setRulesSaving}
      setProvider={setProvider}
      setModelId={setModelId}
      setLevel={setLevel}
      setChoiceDirty={setChoiceDirty}
      setLockBox={setLockBox}
      setLockEdited={setLockEdited}
    />
  );
}
