// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal } from "solid-js";
import { fill, say } from "../../core/lang";
import { FIRST, bandsOf, resultsOf, type Outcome } from "../../core/results";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import Segmented from "../parts/segmented.solid";
import type { Choice } from "../parts/segmented";
import { resultsLookOf } from "./results";
import Look from "./results.look.solid";

type Tab = "all" | Outcome;

export default function CityResults() {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const [tab, setTab] = createSignal<Tab>("all");
  const runs = createMemo(() => Object.values(belief().runs));
  const groups = createMemo(() => resultsOf(runs(), FIRST));
  const listed = createMemo(() => groups().reduce((sum, group) => sum + group.total, 0));
  const tabs = createMemo<readonly Choice<Tab>[]>(() => [
    {
      value: "all",
      label: fill(say(lang(), "results_tab"), {
        name: say(lang(), "results_all"),
        n: String(listed()),
      }),
    },
    ...groups()
      .filter((group) => group.outcome !== "ended" || group.total > 0)
      .map((group) => ({
        value: group.outcome,
        label: fill(say(lang(), "results_tab"), {
          name: say(lang(), `results_${group.outcome}`),
          n: String(group.total),
        }),
      })),
  ]);
  const bands = createMemo(() =>
    bandsOf(
      groups()
        .filter((group) => tab() === "all" || group.outcome === tab())
        .flatMap((group) => group.first),
      u.now(),
    ),
  );
  return (
    <div class="mx-auto w-full max-w-page">
      <p class="text-note text-text-quiet">
        {fill(say(lang(), "results_city"), {
          runs: String(runs().length),
          rest: String(runs().length - listed()),
        })}
      </p>
      <div class="mt-snug">
        <Segmented
          label={say(lang(), "results_which")}
          options={tabs()}
          held={tab()}
          onPick={setTab}
        />
      </div>
      <div class="mt-wide">
        <Look {...resultsLookOf(bands(), lang())} />
      </div>
    </div>
  );
}
