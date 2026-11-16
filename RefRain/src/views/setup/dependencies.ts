// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

export function enablesKey(name: string): Key | null {
  const key = `machine_enables_${name.replaceAll("-", "_")}`;
  return isKey(key) ? key : null;
}

export function stateKey(state: DoctorState): Key {
  if ("present" in state) return "machine_present";
  if ("broken" in state) return "machine_broken";
  return "machine_absent";
}

export function versionOf(state: DoctorState): string | null {
  if (!("present" in state)) return null;
  const said = state.present.version;
  if (typeof said !== "object") return null;
  return /\d+(?:\.\d+)+/u.exec(said.said.text)?.[0] ?? said.said.text;
}

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

export function spelledOf(install: DoctorInstall): string | null {
  if (typeof install === "string") return null;
  if ("command" in install) return install.command.spelled;
  if ("print" in install) return install.print.spelled;
  return install.manual.how;
}

export function ofTier(answer: DoctorAnswer, tier: DoctorTier): readonly DoctorItem[] {
  return answer.items.filter((each) => each.tier === tier);
}

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

export function siteOf(spelled: string): string | null {
  return /https:\/\/([^/\s"]+)/u.exec(spelled)?.[1] ?? null;
}

export function outstanding(answer: DoctorAnswer, tier: DoctorTier): readonly string[] | null {
  return answer.tiers.find((each) => each.tier === tier)?.missing ?? null;
}

export function standing(
  items: readonly DoctorItem[],
  missing: number | null,
): { readonly done: number; readonly total: number } {
  const has = (item: DoctorItem) => "present" in item.state;
  const done = items.filter(has).length;
  const wanted = items.filter((each) => !has(each) && each.need === "optional").length;
  return { done, total: missing === null ? 0 : done + missing + wanted };
}

function runnable(install: DoctorInstall): boolean {
  return typeof install !== "string" && "command" in install;
}

export type Offer = "press" | "by_hand" | "held";

export function offerOf(item: DoctorItem): Offer {
  if ("present" in item.state) return "held";
  return runnable(item.install) ? "press" : "by_hand";
}

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
