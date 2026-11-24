// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, Match, Switch, untrack } from "solid-js";

import { readAnswer } from "../../core/answered";
import { QUERIES } from "../../core/asking";
import { putPreferences } from "../../core/commands";
import { say, type Key } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import {
  CorePlacement,
  CorePriority,
  type AxError,
  type CorePreferences,
  type IdemKey,
  type PreferencePatch,
} from "../../wire";
import Field from "../parts/field.solid";
import Segmented from "../parts/segmented.solid";
import Unanswered from "../parts/unanswered.solid";
import SaveCard from "./save_card_view";
import { edited, RECEIPT_MS, sent, waited, type Saving } from "./saving";

const PLACEMENT: Record<CorePlacement, readonly [Key, Key]> = {
  none: ["performance_none", "performance_none_note"],
  soft: ["performance_soft", "performance_soft_note"],
  soft_shares: ["performance_soft_shares", "performance_soft_shares_note"],
  pinned: ["performance_pinned", "performance_pinned_note"],
};
const PRIORITY: Record<CorePriority, readonly [Key, Key]> = {
  raised: ["performance_raised", "performance_raised_note"],
  normal: ["performance_normal", "performance_normal_note"],
};

interface Draft {
  readonly placement: CorePlacement;
  readonly priority: CorePriority;
  readonly memory: string;
}

interface SavingBatch {
  readonly requests: readonly IdemKey[];
  readonly expected: CorePreferences;
  readonly before: CorePreferences;
}

function fromCore(core: CorePreferences | undefined): Draft | undefined {
  return core?.placement === undefined || core.priority === undefined
    ? undefined
    : {
        placement: core.placement,
        priority: core.priority,
        memory: String(core.memory_bytes ?? ""),
      };
}

function memoryAmount(memory: string): number | null | undefined {
  const input = memory.trim();
  if (input === "") return null;
  const bytes = Number(input);
  return /^[1-9][0-9]*$/.test(input) && Number.isSafeInteger(bytes) ? bytes : undefined;
}

function changed(draft: Draft, core: CorePreferences): boolean {
  return (
    draft.placement !== core.placement ||
    draft.priority !== core.priority ||
    memoryAmount(draft.memory) !== (core.memory_bytes ?? null)
  );
}

function rebase(
  draft: Draft | undefined,
  previous: CorePreferences | undefined,
  actual: CorePreferences,
  issued: CorePreferences | undefined,
): Draft | undefined {
  const next = fromCore(actual);
  if (next === undefined || draft === undefined || previous === undefined) return next;
  return {
    placement:
      issued?.placement !== undefined || draft.placement !== previous.placement
        ? draft.placement
        : next.placement,
    priority:
      issued?.priority !== undefined || draft.priority !== previous.priority
        ? draft.priority
        : next.priority,
    memory:
      issued?.memory_bytes !== undefined ||
      memoryAmount(draft.memory) !== (previous.memory_bytes ?? null)
        ? draft.memory
        : next.memory,
  };
}

function confirmed(actual: CorePreferences, expected: CorePreferences): boolean {
  return (
    (expected.placement === undefined || actual.placement === expected.placement) &&
    (expected.priority === undefined || actual.priority === expected.priority) &&
    (expected.memory_bytes === undefined || (actual.memory_bytes ?? null) === expected.memory_bytes)
  );
}

export default function Performance() {
  const u = ui();
  const lang = storeValue(u.lang);
  const asked = storeValue(u.conn.asking.ask(QUERIES.preferences));
  const receipts = storeValue(u.conn.receipts);
  const read = createMemo(() =>
    readAnswer(asked(), (answer) =>
      "preferences" in answer ? answer.preferences.core : undefined,
    ),
  );
  const core = () => {
    const answer = read();
    return answer.kind === "held" ? answer.value : undefined;
  };
  const unavailable = () => {
    const answer = read();
    return answer.kind === "unavailable" ? answer.query : undefined;
  };
  const [draft, setDraft] = createSignal<Draft>();
  const [pending, setPending] = createSignal<SavingBatch>();
  const [unknown, setUnknown] = createSignal(false);
  const [saved, setSaved] = createSignal(false);
  const [failure, setFailure] = createSignal<AxError>();
  let prior: CorePreferences | undefined;

  const word = (key: Key) => say(lang(), key);
  const invalid = () => draft() !== undefined && memoryAmount(draft()?.memory ?? "") === undefined;
  const refresh = () => u.conn.asking.refresh(QUERIES.preferences);

  const status = createMemo<Saving>(() => {
    const operation = pending();
    if (operation !== undefined) {
      const issued = sent(JSON.stringify(operation.before));
      return unknown() ? waited(issued) : issued;
    }
    const error = failure();
    if (error !== undefined) return { kind: "refused", error };
    const value = draft();
    const actual = core() ?? prior;
    if (value !== undefined && actual !== undefined && changed(value, actual)) return edited(true);
    return saved() ? { kind: "saved" } : edited(false);
  });

  createEffect(
    () => ({ core: core(), operation: pending(), receipts: receipts() }),
    ({ core: actual, operation, receipts }) => {
      const value = untrack(draft);
      if (operation !== undefined) {
        const error = operation.requests
          .map((key) => receipts.get(key))
          .find((receipt) => receipt?.kind === "refused");
        if (error?.kind === "refused") {
          setFailure(error.error);
          setPending(undefined);
          refresh();
        } else if (
          actual !== undefined &&
          actual !== operation.before &&
          confirmed(actual, operation.expected)
        ) {
          setSaved(true);
          setPending(undefined);
        }
      }
      if (actual === undefined || actual === prior) return;
      setDraft(rebase(value, prior, actual, operation?.expected));
      prior = actual;
    },
  );

  createEffect(
    () => pending(),
    (operation) => {
      if (operation === undefined) return;
      const check = window.setInterval(refresh, 1_000);
      const timeout = window.setTimeout(() => {
        window.clearInterval(check);
        setUnknown(true);
      }, RECEIPT_MS);
      return () => {
        window.clearInterval(check);
        window.clearTimeout(timeout);
      };
    },
  );

  function edit(value: Draft): void {
    setDraft(value);
    setFailure(undefined);
    setSaved(false);
  }

  function save(): void {
    const actual = core();
    const value = draft();
    if (actual === undefined || value === undefined || pending() !== undefined) return;
    const bytes = memoryAmount(value.memory);
    if (bytes === undefined) return;
    const requests: IdemKey[] = [];
    let expected: CorePreferences = {};
    function issue(patch: PreferencePatch, matching: CorePreferences): void {
      const command = putPreferences(patch);
      if (!("put_preferences" in command) || !u.send(command)) return;
      requests.push(command.put_preferences.idem);
      expected = { ...expected, ...matching };
    }
    if (value.placement !== actual.placement)
      issue({ core_placement: value.placement }, { placement: value.placement });
    if (value.priority !== actual.priority)
      issue({ core_priority: value.priority }, { priority: value.priority });
    if (bytes !== (actual.memory_bytes ?? null))
      issue({ run_memory: bytes }, { memory_bytes: bytes });
    if (requests.length === 0) return;
    setFailure(undefined);
    setSaved(false);
    setUnknown(false);
    setPending({ requests, expected, before: actual });
    refresh();
  }

  return (
    <Switch>
      <Match when={draft()}>
        {(current) => (
          <SaveCard
            title={word("performance_core")}
            note={word("performance_core_note")}
            saving={status()}
            settled={word("performance_restart")}
            blocked={
              invalid() && status().kind !== "unverified" ? "performance_memory_invalid" : undefined
            }
            onSave={save}
            onCheck={refresh}
          >
            <Segmented
              label={word("performance_placement")}
              held={current().placement}
              options={CorePlacement.literals.map((value) => ({
                value,
                label: word(PLACEMENT[value][0]),
              }))}
              onPick={(placement) => edit({ ...current(), placement })}
            />
            <p class="text-note text-text-quiet">{word(PLACEMENT[current().placement][1])}</p>
            <Segmented
              label={word("performance_priority")}
              held={current().priority}
              options={CorePriority.literals.map((value) => ({
                value,
                label: word(PRIORITY[value][0]),
              }))}
              onPick={(priority) => edit({ ...current(), priority })}
            />
            <p class="text-note text-text-quiet">{word(PRIORITY[current().priority][1])}</p>
            <Field
              label={word("performance_memory")}
              value={current().memory}
              help={word("performance_memory_note")}
              {...(invalid() ? { error: word("performance_memory_invalid") } : {})}
              onInput={(memory) => edit({ ...current(), memory })}
              mono
            />
          </SaveCard>
        )}
      </Match>
      <Match when={unavailable()}>
        {(query) => <Unanswered query={query()} asked={QUERIES.preferences} />}
      </Match>
    </Switch>
  );
}
