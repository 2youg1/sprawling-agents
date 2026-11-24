// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo } from "solid-js";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Address, CityAnswer } from "../../wire";
import type { BoardRun } from "../runs/lineage";
import { lookOf } from "./table";
import Look from "./table.look.solid";

export interface CityTableProps {
  readonly city: CityAnswer;
  readonly runs: readonly BoardRun[];
  readonly now: number;
  readonly picked: Address | null;
  readonly onPick: (at: Address) => void;
}

export default function CityTable(props: CityTableProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const editor = storeValue(u.prefs.editorHeld);
  const look = createMemo(() =>
    lookOf(
      {
        city: props.city,
        runs: props.runs,
        now: props.now,
        picked: props.picked,
        lang: lang(),
        cityFolder: editor().folder,
      },
      (addr) => props.onPick(addr),
    ),
  );
  return <Look {...look()} />;
}
