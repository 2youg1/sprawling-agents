// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the privacy page draws from one answer of the host
// (`Query::Privacy`), decided without the DOM: the sections in page order,
// which action each entry offers and with which expected value, the
// result of an operation this page sent, and the words each entry's look
// is given. The host decides every fact (targets, written values,
// edition fit); this module only reads them, so the page cannot hold a
// second opinion about a control.
//
// An entry offers nothing it cannot do, and says why on the page:
// nothing while this app's history is withheld or unreadable, only the
// check while one change has no conclusion (every write waits for it),
// apply while the value read differs from the value written, restore
// while the value read is still the one this app wrote.

import { fill, say, type Key, type Lang } from "../../../core/lang";
import type {
  IdemKey,
  PrivacyAction,
  PrivacyAnswer,
  PrivacyCategory,
  PrivacyControl,
  PrivacyControlEntry,
  PrivacyCurrent,
  PrivacyHistory,
  PrivacyHost,
  PrivacyIntent,
  PrivacyResult,
  PrivacyTarget,
  PrivacyValue,
} from "../../../wire";
import type { Weight } from "../../parts/glyph";
import type { Action, EntryLook, Fact, Press, Result, Stage } from "./entry";
import { categoryWord, controlWord, editionWord, effectWord, fitWord } from "./words";

export interface Offer {
  readonly action: Action;
  // The value the page shows now, sent back unchanged as `expected`.
  readonly expected: PrivacyValue;
}

export interface EntryModel {
  readonly row: PrivacyControlEntry;
  // The latest change of this control this app still owns.
  readonly owned: PrivacyIntent | null;
  readonly offers: readonly Offer[];
}

export interface Section {
  readonly category: PrivacyCategory;
  readonly entries: readonly EntryModel[];
}

// What this app's history lets the page do, in the order it is checked.
export type Standing = "withheld" | "unreadable" | "unresolved" | "open";

export function standingOf(history: PrivacyHistory): Standing {
  if ("withheld" in history) return "withheld";
  if ("unreadable" in history) return "unreadable";
  return history.disclosed.unresolved == null ? "open" : "unresolved";
}

// The entries, grouped by category in the order the host sends them,
// which is page order.
export function sectionsOf(answer: PrivacyAnswer): readonly Section[] {
  const sections: { category: PrivacyCategory; entries: EntryModel[] }[] = [];
  for (const row of answer.controls) {
    const model: EntryModel = {
      row,
      owned: ownedOf(answer.history, row.control),
      offers: offersOf(row, answer.history),
    };
    const last = sections.at(-1);
    if (last?.category === row.category) last.entries.push(model);
    else sections.push({ category: row.category, entries: [model] });
  }
  return sections;
}

function ownedOf(history: PrivacyHistory, control: PrivacyControl): PrivacyIntent | null {
  return "disclosed" in history ? (history.disclosed.owned.find((each) => each.control === control) ?? null) : null;
}

export function offersOf(row: PrivacyControlEntry, history: PrivacyHistory): readonly Offer[] {
  const value = readOf(row.current);
  if (value === null || !("disclosed" in history)) return [];
  const { owned, unresolved } = history.disclosed;
  if (unresolved != null) {
    // wording-ok: the wire name of a privacy action, not a word for a reader
    return unresolved.control === row.control ? [{ action: "reconcile", expected: value }] : [];
  }
  const offers: Offer[] = [];
  const written = row.written ?? null;
  // wording-ok: the wire name of a privacy action, not a word for a reader
  if (written !== null && !sameValue(value, written)) offers.push({ action: "apply", expected: value });
  const mine = owned.find((each) => each.control === row.control);
  // wording-ok: the wire name of a privacy action, not a word for a reader
  if (mine !== undefined && sameValue(value, mine.modified)) offers.push({ action: "restore", expected: value });
  return offers;
}

function readOf(current: PrivacyCurrent): PrivacyValue | null {
  return typeof current !== "string" && "read" in current ? current.read.value : null;
}

// The command for one offer of one control.
export function actionOf(control: PrivacyControl, offer: Offer): PrivacyAction {
  switch (offer.action) {
    case "apply":
      return { apply: { control, expected: offer.expected } };
    case "restore":
      return { restore: { control, expected: offer.expected } };
    case "reconcile":
      return { reconcile: { expected: offer.expected } };
  }
}

// The kept result of the operation sent under `idem`, if the host still
// keeps it. Only an idem this page minted is ever asked about.
export function outcomeOf(answer: PrivacyAnswer, idem: IdemKey): PrivacyResult | null {
  return answer.outcomes.find((each) => each.idem === idem)?.result ?? null;
}

export function resultOf(shown: PrivacyResult): Result {
  if (shown === "running") return "running";
  if (shown !== "already_written" && "refused" in shown) return "refused";
  return "done";
}

// Two values are the same exactly when they are spelled the same: the
// wire gives every value one spelling (wire D49).
export function sameValue(a: PrivacyValue, b: PrivacyValue): boolean {
  switch (a.value) {
    case "absent":
    case "task_absent":
      return b.value === a.value;
    case "dword":
      return b.value === "dword" && b.number === a.number;
    case "text":
      return b.value === "text" && b.text === a.text;
    case "raw":
      return b.value === "raw" && b.kind === a.kind && b.hex === a.hex;
    case "task_enabled":
      return b.value === "task_enabled" && b.definition_sha256 === a.definition_sha256;
    case "task_disabled":
      return b.value === "task_disabled" && b.definition_sha256 === a.definition_sha256;
  }
}

export function valueWord(lang: Lang, value: PrivacyValue): string {
  switch (value.value) {
    case "absent":
      return say(lang, "privacy_value_absent");
    case "dword":
      return String(value.number);
    case "text":
      return value.text;
    case "raw":
      return fill(say(lang, "privacy_value_raw"), { kind: String(value.kind), hex: value.hex });
    case "task_absent":
      return say(lang, "privacy_value_task_absent");
    case "task_enabled":
      return say(lang, "privacy_value_task_enabled");
    case "task_disabled":
      return say(lang, "privacy_value_task_disabled");
  }
}

function currentWord(lang: Lang, current: PrivacyCurrent): string {
  if (current === "access_denied") return say(lang, "privacy_current_access_denied");
  if (current === "not_read") return say(lang, "privacy_current_not_read");
  if ("failed" in current) return fill(say(lang, "privacy_current_failed"), { why: current.failed.error.recovery });
  return valueWord(lang, current.read.value);
}

// The full place a control writes, as Windows tools spell it.
export function targetPath(target: PrivacyTarget): string {
  switch (target.kind) {
    case "registry_value_hklm":
      return `HKLM\\${target.path}\\${target.name}`;
    case "registry_value_hkcu":
      return `HKCU\\${target.path}\\${target.name}`;
    case "environment_variable_user":
      return `HKCU\\Environment\\${target.name}`;
    case "scheduled_task_enabled":
      return `${target.path}${target.name}`;
  }
}

// The host's facts, one labelled reading each, or the one sentence that
// says why there are none.
export function hostFacts(lang: Lang, host: PrivacyHost): readonly Fact[] | string {
  if (host === "not_windows") return say(lang, "privacy_host_not_windows");
  if ("unreadable" in host) return say(lang, "privacy_host_unreadable");
  const { windows } = host;
  const edition =
    windows.edition == null
      ? fill(say(lang, "privacy_host_edition_unlisted"), { id: windows.edition_id })
      : `${say(lang, editionWord(windows.edition))} (${windows.edition_id})`;
  return [
    { label: say(lang, "privacy_host_version"), value: windows.display_version ?? say(lang, "privacy_host_unrecorded"), figure: true },
    { label: say(lang, "privacy_host_edition"), value: edition, figure: false },
    { label: say(lang, "privacy_host_build"), value: windows.build, figure: true },
  ];
}

export function sectionWord(category: PrivacyCategory): Key {
  return categoryWord(category);
}

const PRESS: Record<Action, Key> = {
  apply: "privacy_apply",
  restore: "privacy_restore",
  reconcile: "privacy_reconcile",
};

export function pressWord(action: Action): Key {
  return PRESS[action];
}

const FIT: Record<PrivacyControlEntry["host_fit"], Weight> = {
  honoured: "quiet",
  not_stated: "quiet",
  ignored: "alert",
};

// What the page says about the last operation it sent on an entry.
function statusOf<I>(
  lang: Lang,
  row: PrivacyControlEntry,
  stage: Stage<I>,
  shown: PrivacyResult | null,
): EntryLook["status"] {
  switch (stage.kind) {
    case "idle":
    case "confirming":
      return null;
    case "sending":
      return {
        text: say(lang, row.scope === "machine" ? "privacy_waiting_machine" : "privacy_waiting"),
        weight: "live",
      };
    case "unsent":
      return { text: say(lang, "privacy_unsent"), weight: "alert" };
    case "settled":
    case "refused":
      return shown === null ? null : settledStatus(lang, shown);
  }
}

function settledStatus(lang: Lang, shown: PrivacyResult): EntryLook["status"] {
  if (shown === "running") return { text: say(lang, "privacy_waiting"), weight: "live" };
  if (shown === "already_written") return { text: say(lang, "privacy_result_already"), weight: "quiet" };
  if ("applied" in shown) return { text: say(lang, "privacy_result_applied"), weight: "quiet" };
  if ("restored" in shown) return { text: say(lang, "privacy_result_restored"), weight: "quiet" };
  if ("reconciled" in shown) {
    const settlement: Key = `privacy_settlement_${shown.reconciled.settlement}`;
    return { text: fill(say(lang, "privacy_result_reconciled"), { settlement: say(lang, settlement) }), weight: "quiet" };
  }
  return { text: fill(say(lang, "privacy_result_refused"), { why: shown.refused.error.recovery }), weight: "alert" };
}

// The id an entry carries under the id root of the page drawing it, so a
// line not written can point at it and two pages never share an id.
export function entryId(root: string, control: PrivacyControl): string {
  return `${root}-${control}`;
}

export interface Drawn<I> {
  readonly root: string;
  readonly stage: Stage<I>;
  readonly shown: PrivacyResult | null;
  readonly press: (action: Action) => void;
}

// The whole look of one entry in the person's language.
export function lookOf<I>(lang: Lang, model: EntryModel, drawn: Drawn<I>): EntryLook {
  const { row, owned } = model;
  const word = (field: Parameters<typeof controlWord>[1]) => say(lang, controlWord(row.control, field));
  const list = (editions: readonly PrivacyControlEntry["editions"]["honoured"][number][]) =>
    editions.length === 0 ? say(lang, "privacy_none_listed") : editions.map((each) => say(lang, editionWord(each))).join(", ");
  const presses: Press[] = model.offers.map((offer) => ({
    label: say(lang, pressWord(offer.action)),
    onPress: () => {
      drawn.press(offer.action);
    },
  }));
  return {
    id: entryId(drawn.root, row.control),
    title: word("title"),
    lines:
      row.originals.length === 0
        ? { label: say(lang, "privacy_lines"), text: say(lang, "privacy_added") }
        : { label: say(lang, "privacy_lines"), text: row.originals.map((line) => line.text).join("  ·  ") },
    notes: [
      { label: say(lang, "privacy_does"), text: word("does") },
      { label: say(lang, "privacy_benefit"), text: word("benefit") },
      { label: say(lang, "privacy_affects"), text: word("affects") },
    ],
    overlooked: { label: say(lang, "privacy_overlooked"), text: word("overlooked") },
    editions: {
      label: say(lang, "privacy_editions"),
      text: fill(say(lang, "privacy_editions_lists"), {
        honoured: list(row.editions.honoured),
        ignored: list(row.editions.ignored),
      }),
    },
    editionsNote: word("editions"),
    fit: say(lang, fitWord(row.host_fit)),
    fitWeight: FIT[row.host_fit],
    effect: row.build_effect === "documented" ? null : say(lang, effectWord(row.build_effect)),
    facts: [
      { label: say(lang, "privacy_current"), value: currentWord(lang, row.current), figure: true },
      {
        label: say(lang, "privacy_written"),
        value: row.written == null ? say(lang, "privacy_written_unknown") : valueWord(lang, row.written),
        figure: true,
      },
      {
        label: say(lang, "privacy_original"),
        value: owned === null ? say(lang, "privacy_not_changed") : valueWord(lang, owned.original),
        figure: owned !== null,
      },
      { label: say(lang, "privacy_scope"), value: say(lang, `privacy_scope_${row.scope}`), figure: false },
    ],
    target: { label: say(lang, "privacy_target"), text: targetPath(row.target) },
    presses,
    status: statusOf(lang, row, drawn.stage, drawn.shown),
  };
}

// What the confirmation states before the click: the change, whose
// settings it is, what a person could overlook and how it is undone, so
// an entry that deletes data or needs a restart says so first.
export interface ConfirmLook {
  readonly title: string;
  readonly confirmLabel: string;
  readonly facts: readonly Fact[];
  readonly notes: readonly { readonly label: string; readonly text: string }[];
}

export function confirmOf(lang: Lang, model: EntryModel, action: Action): ConfirmLook {
  const { row, owned } = model;
  const title = say(lang, controlWord(row.control, "title"));
  const now = currentWord(lang, row.current);
  const to = (): string => {
    switch (action) {
      case "apply":
        return row.written == null ? say(lang, "privacy_written_unknown") : valueWord(lang, row.written);
      case "restore":
        return owned === null ? say(lang, "privacy_not_changed") : valueWord(lang, owned.original);
      case "reconcile":
        return now;
    }
  };
  const confirmTitle: Key = `privacy_confirm_${action}`;
  return {
    title: fill(say(lang, confirmTitle), { title }),
    confirmLabel: say(lang, pressWord(action)),
    facts: [
      { label: say(lang, "privacy_current"), value: now, figure: true },
      { label: say(lang, action === "reconcile" ? "privacy_recorded" : "privacy_after"), value: to(), figure: true },
      { label: say(lang, "privacy_scope"), value: say(lang, `privacy_scope_${row.scope}`), figure: false },
    ],
    notes:
      action === "reconcile"
        ? [{ label: say(lang, "privacy_reconcile_label"), text: say(lang, "privacy_reconcile_detail") }]
        : [
            { label: say(lang, "privacy_overlooked"), text: say(lang, controlWord(row.control, "overlooked")) },
            { label: say(lang, "privacy_undo"), text: say(lang, controlWord(row.control, "undo")) },
          ],
  };
}
