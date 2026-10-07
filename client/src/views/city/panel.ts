// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the picked building's panel says, already in words: how far the
// plan has got, what is ready, what is stuck and why, the standing goal,
// and the newest runs that worked there. The look draws `PanelLook` and
// decides nothing (client D95).

import type { RunBelief } from "../../core/belief";
import type { Lang } from "../../core/lang";
import { fill, say } from "../../core/lang";
import { toFragment } from "../../core/route";
import { clock } from "../../core/time";
import type { Address, CityAnswer } from "../../wire";
import type { Weight } from "../parts/glyph";

// How many of the building's runs the panel lists, newest first.
const RUNS = 8;

export interface PanelLook {
  readonly name: string;
  readonly close: { readonly label: string; readonly onPress: () => void };
  // The plan's progress, what is ready and what is stuck, each a badge
  // at its tier.
  readonly badges: readonly { readonly key: string; readonly text: string; readonly weight: Weight }[];
  // Spread on the link into the building's page.
  readonly enter: { readonly label: string; readonly wire: { readonly href: string } };
  readonly pursuit: { readonly goal: string; readonly running: boolean } | null;
  readonly problems: readonly string[];
  readonly blocked: readonly { readonly key: string; readonly source: string; readonly line: string }[];
  readonly runs: {
    readonly heading: string;
    readonly none: string;
    readonly rows: readonly {
      readonly key: string;
      readonly title: string;
      readonly moving: boolean;
      readonly at: string | null;
      // Spread on the line's link to the run's page.
      readonly wire: { readonly href: string };
    }[];
  };
}

export interface PanelSeat {
  readonly addr: Address;
  readonly city: CityAnswer;
  // Every run the belief holds in the building, oldest first.
  readonly held: readonly RunBelief[];
  readonly lang: Lang;
}

export function panelLookOf(seat: PanelSeat, onClose: () => void): PanelLook {
  const { addr, lang } = seat;
  const building = seat.city.buildings.find((each) => each.addr === addr);
  const pursuit = seat.city.pursuits.find((line) => line.addr === addr);
  return {
    name: addr,
    close: { label: say(lang, "panel_close"), onPress: onClose },
    badges: building === undefined ? [] : badgesOf(building, lang),
    enter: { label: say(lang, "city_enter"), wire: { href: toFragment({ kind: "building", address: addr }) } },
    pursuit: pursuit === undefined ? null : { goal: pursuit.goal, running: pursuit.state === "running" },
    problems: building?.problems ?? [],
    blocked: (building?.blocked ?? []).map((line) => ({ key: `${line.source}\u0000${line.line}`, source: line.source, line: line.line })),
    runs: {
      heading: say(lang, "city_runs"),
      none: say(lang, "tree_no_runs"),
      rows: [...seat.held]
        .reverse()
        .slice(0, RUNS)
        .map((run) => ({
          key: run.run,
          title: run.task ?? run.run,
          moving: run.doing.kind !== "frozen",
          at: run.started === null ? null : clock(lang, run.started),
          wire: { href: toFragment({ kind: "run", run: run.run }) },
        })),
    },
  };
}

// A building with no plan has no denominator, so it carries no
// progress badge rather than a zero that would read as a plan nobody
// has started; what is ready is said only of a planned building.
function badgesOf(building: CityAnswer["buildings"][number], lang: Lang): PanelLook["badges"] {
  const planned = "planned" in building.progress ? building.progress.planned : null;
  const progress =
    planned === null
      ? []
      : [{ key: "progress", text: fill(say(lang, "bld_progress"), { done: String(planned.done), total: String(planned.total) }), weight: "quiet" as const }];
  const ready =
    planned !== null && building.ready > 0
      ? [{ key: "ready", text: fill(say(lang, "city_ready"), { n: String(building.ready) }), weight: "live" as const }]
      : [];
  const stuck =
    building.blocked.length > 0
      ? [{ key: "stuck", text: fill(say(lang, "city_stuck"), { n: String(building.blocked.length) }), weight: "alert" as const }]
      : [];
  return [...progress, ...ready, ...stuck];
}
