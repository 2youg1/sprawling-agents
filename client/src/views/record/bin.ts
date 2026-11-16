// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Lang } from "../../core/lang";
import { say } from "../../core/lang";
import { clock } from "../../core/time";
import type { DiscardLine, Restoration } from "../../wire";

export type Standing = "restored" | "gone";

export interface RestoreLook {
  readonly label: string;
  readonly onPress: () => void;
}

export interface BinRowLook {
  readonly key: string;
  readonly path: string;
  readonly when: string;
  readonly standing: Standing;
  readonly standingWord: string;

  readonly restore: RestoreLook | undefined;
  readonly way: string;
}

export interface BinLook {
  readonly rows: readonly BinRowLook[];
}

export type Restore = (restoration: Restoration) => void;

export function lookOf(rows: readonly DiscardLine[], lang: Lang, restore: Restore): BinLook {
  return { rows: rows.map((row) => rowOf(row, lang, restore)) };
}

function rowOf(row: DiscardLine, lang: Lang, restore: Restore): BinRowLook {
  return {

    key: `${String(row.at)}\u0000${row.path}`,
    path: row.path,
    when: clock(lang, row.at),
    standing: row.restored ? "restored" : "gone",
    standingWord: say(lang, row.restored ? "bin_restored" : "bin_gone"),
    restore: restoreOf(row, lang, restore),
    way: wayOf(row.restoration, lang),
  };
}

function restoreOf(row: DiscardLine, lang: Lang, restore: Restore): RestoreLook | undefined {
  const restoration = row.restoration;
  if (
    row.restored ||
    restoration === null ||
    restoration === undefined ||
    !("tracked" in restoration)
  ) {
    return undefined;
  }
  return {
    label: say(lang, "bin_restore"),
    onPress: () => {
      restore(restoration);
    },
  };
}

function wayOf(restoration: Restoration | null | undefined, lang: Lang): string {
  if (restoration === null || restoration === undefined) return "";
  if ("tracked" in restoration) return `${say(lang, "bin_tracked")} ${restoration.tracked}`;
  if ("interred" in restoration) return `${say(lang, "bin_interred")} ${restoration.interred}`;
  return `${say(lang, "bin_rebuildable")} ${restoration.rebuildable.reason}`;
}
