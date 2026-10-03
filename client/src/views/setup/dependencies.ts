// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the dependencies group reads out of the city's answer about this
// machine: one word per program, the one command that would get it,
// which tier a program belongs to, and how far a tier has got.
//
// These are readings rather than rendering. The card draws them and the
// gallery fixture hands them their answer; a second spelling of what a
// state means would disagree with this one the day a fourth state
// arrives. The view above them decides only how loud each answer is
// drawn.

import { isKey, type Key } from "../../core/lang";
import type {
  DoctorAbsence,
  DoctorAnswer,
  DoctorFault,
  DoctorInstall,
  DoctorItem,
  DoctorState,
  DoctorTier,
  DoctorVersion,
} from "../../wire";

// What the item is for, in the page's words: the wire carries the item's
// name as its id, and the clause lives in `lang.json` under
// `machine_enables_<name>`, a hyphen in the name spelled `_` because every
// key is snake_case (sprawling D6). An item this client has
// no clause for is shown by its name alone.
export function enablesKey(name: string): Key | null {
  const key = `machine_enables_${name.replaceAll("-", "_")}`;
  return isKey(key) ? key : null;
}

// The one word a card is labelled by. The state is a value the city
// sent; every word around it comes from the phrase table.
export function stateKey(
  state: DoctorState,
): Key {
  if ("present" in state) return "machine_present";
  if ("broken" in state) return "machine_broken";
  return "machine_absent";
}

// What a present item said when asked its version, when it said
// anything a person can read. The three wordless answers - it said
// nothing, its line was not text, it was late - are facts about how it
// did not answer, and a card shows the item rather than them.
//
// The number out of the line rather than the line: `git version 2.55.0
// .windows.1` and a path uv printed both carry one, and a row has room
// for the number alone. A line with no dotted number is shown as it is.
export function versionOf(state: DoctorState): string | null {
  if (!("present" in state)) return null;
  const said = state.present.version;
  if (typeof said !== "object") return null;
  return /\d+(?:\.\d+)+/u.exec(said.said.text)?.[0] ?? said.said.text;
}

// Why the state is what it is, in one phrase, when the state alone does
// not say it: a present program that gave no version, a broken one and
// its fault, an absent one and where the city looked. `said` is the
// program's or the platform's own words, or a path, shown as they came.
export interface Reason {
  readonly key: Key;
  readonly said: string | null;
}

export function reasonOf(state: DoctorState): Reason | null {
  if ("present" in state) return wordless(state.present.version);
  if ("broken" in state) return faultOf(state.broken.fault);
  return absenceReason(state.absent.absence);
}

function wordless(version: DoctorVersion): Reason | null {
  if (typeof version === "object") return null;
  switch (version) {
    case "silent":
      return { key: "machine_version_silent", said: null };
    case "unreadable":
      return { key: "machine_version_unreadable", said: null };
    case "late":
      return { key: "machine_version_late", said: null };
  }
}

function faultOf(fault: DoctorFault): Reason {
  if (typeof fault === "string") return { key: "machine_fault_half_written", said: null };
  return "will_not_start" in fault
    ? { key: "machine_fault_will_not_start", said: fault.will_not_start.said }
    : { key: "machine_fault_unreadable", said: fault.unreadable.said };
}

function absenceReason(absence: DoctorAbsence): Reason {
  if (typeof absence === "object") {
    return "variable_names_nothing" in absence
      ? {
          key: "machine_absence_variable",
          said: `${absence.variable_names_nothing.variable}=${absence.variable_names_nothing.path}`,
        }
      : { key: "machine_absence_component", said: absence.no_component.dir };
  }
  switch (absence) {
    case "not_on_search_path":
      return { key: "machine_absence_search_path", said: null };
    case "no_home":
      return { key: "machine_absence_no_home", said: null };
    case "not_in_this_build":
      return { key: "machine_absence_build", said: null };
  }
}

// One row of the dependency list: every item, here or not,
// says its state, why when the state alone does not, the version it gave,
// and how to get it when this machine lacks it. The card draws exactly
// these four, so a row cannot leave one out for one state and show it
// for another.
export interface Row {
  readonly state: Key;
  readonly reason: Reason | null;
  readonly version: string | null;
  readonly install: string | null;
  readonly offer: Offer;
}

export function rowOf(item: DoctorItem): Row {
  const offer = offerOf(item);
  return {
    state: stateKey(item.state),
    reason: reasonOf(item.state),
    version: versionOf(item.state),
    install: offer === "held" ? null : spelledOf(item.install),
    offer,
  };
}

// The command that would get a missing item, as a person would type it.
export function spelledOf(install: DoctorInstall): string | null {
  if (typeof install === "string") return null;
  if ("command" in install) return install.command.spelled;
  if ("print" in install) return install.print.spelled;
  return install.manual.how;
}

// The rows of one tier, in the city's table order, which for the develop
// tier is the order installing them has to follow.
export function ofTier(answer: DoctorAnswer, tier: DoctorTier): readonly DoctorItem[] {
  return answer.items.filter((each) => each.tier === tier);
}

// Where a command the city may run gets its program from, named by the
// program the command starts. A program this page has no name for is
// shown as itself.
const SOURCE: Readonly<Partial<Record<string, Key>>> = {
  winget: "machine_source_winget",
  brew: "machine_source_brew",
  cargo: "machine_source_cargo",
  rustup: "machine_source_rustup",
  elan: "machine_source_elan",
  uv: "machine_source_uv",
};

export function sourceOf(spelled: string): { readonly key: Key } | { readonly program: string } {
  const program = spelled.split(" ")[0] ?? spelled;
  const key = SOURCE[program];
  return key === undefined ? { program } : { key };
}

// The site a printed install script is downloaded from, when its line
// names one.
export function siteOf(spelled: string): string | null {
  return /https:\/\/([^/\s"]+)/u.exec(spelled)?.[1] ?? null;
}

// What the city still names as missing for one tier, or nothing when
// it has answered about no such tier.
//
// The names rather than the count of them: the tier states both, and a
// count taken here and a list taken somewhere else would be two
// readings of one verdict.
export function outstanding(answer: DoctorAnswer, tier: DoctorTier): readonly string[] | null {
  return answer.tiers.find((each) => each.tier === tier)?.missing ?? null;
}

// How far a tier has got, counted the way the city counts it.
//
// **The cards are not the authority for what is missing.** A tier's
// verdict collapses a run of interchangeable items - any one browser
// engine will do - into the single thing a person is still missing, so
// a machine with Firefox on it is missing no browser however many of
// the other cards stay empty; counting cards was what kept that column
// short of full for somebody who had everything.
//
// The denominator is therefore what this machine has, plus what the
// city still names for the tier, plus the optional rows it does not
// have: an optional item counts against no tier, and is still a row a
// person can act on. The bar fills exactly when nothing on the tier is
// left to do.
//
// A tier the city said nothing about leaves the end unknown, and
// `Progress` draws that as busy rather than as a fraction.
export function standing(
  items: readonly DoctorItem[],
  missing: number | null,
): { readonly done: number; readonly total: number } {
  const has = (item: DoctorItem) => "present" in item.state;
  const done = items.filter(has).length;
  const wanted = items.filter((each) => !has(each) && each.need === "optional").length;
  return { done, total: missing === null ? 0 : done + missing + wanted };
}

// Whether this city may run the install itself. The other two recipes
// are a command the person runs and an instruction they follow, and
// both stay a copy rather than a button.
function runnable(install: DoctorInstall): boolean {
  return typeof install !== "string" && "command" in install;
}

// What the install control offers for one item.
//
// One reading of the item, so the colour, the reason and the press can
// never disagree: what this machine already has is not offered in the
// colour reserved for the one action a screen is for.
export type Offer = "press" | "by_hand" | "held";

export function offerOf(item: DoctorItem): Offer {
  if ("present" in item.state) return "held";
  return runnable(item.install) ? "press" : "by_hand";
}

// Whether an absent required card is one the tier still waits for, or a
// spare the tier has done without.
//
// The wire spells an interchangeable member - any one browser driver
// will do - as `required`, and the city's verdict names such a group
// under the group's own name rather than the card's. So an absent
// required card the verdict does not name is a member of a group, and
// when every name the verdict gives is a card of its own, no group is
// still short: the card is a spare, drawn quiet and captioned with the
// rule instead of the warning a card the city waits for carries.
export type Absence = "wanted" | "spare";

export function absenceOf(
  item: DoctorItem,
  missing: readonly string[] | null,
  items: readonly DoctorItem[],
): Absence {
  if (!("absent" in item.state) || missing === null || missing.includes(item.name)) return "wanted";
  const named = (name: string) => items.some((each) => each.name === name);
  return missing.every(named) ? "spare" : "wanted";
}
