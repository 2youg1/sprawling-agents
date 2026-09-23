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

import type { Key } from "../../core/lang";
import type {
  DoctorAnswer,
  DoctorInstall,
  DoctorItem,
  DoctorState,
  DoctorTier,
} from "../../wire";

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
export function versionOf(state: DoctorState): string | null {
  if (!("present" in state)) return null;
  const said = state.present.version;
  return typeof said === "object" ? said.said.text : null;
}

// The command that would get a missing item, as a person would type it.
export function spelledOf(install: DoctorInstall): string | null {
  if (typeof install === "string") return null;
  if ("command" in install) return install.command.spelled;
  if ("print" in install) return install.print.spelled;
  return install.manual.how;
}

// What the city cannot run without. Everything else - the optional
// members of the run tier and the whole develop tier - is a
// recommendation rather than a requirement.
export function required(answer: DoctorAnswer): readonly DoctorItem[] {
  return answer.items.filter((each) => each.tier === "use" && each.need === "required");
}

export function recommended(answer: DoctorAnswer): readonly DoctorItem[] {
  return answer.items.filter((each) => each.tier !== "use" || each.need !== "required");
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
