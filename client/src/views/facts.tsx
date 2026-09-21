// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The strip along the bottom of every page, and the one home of the
// seven facts a person needs while something is being done on their
// behalf.
//
// **Four of the seven are risk readings** - what this has cost, how far
// the door is open, whether the work is boxed in, and whether the page
// is still hearing from the city. A risk reading has to be visible when
// nobody is looking for it. Put behind a page somebody has to open, it
// only works after they have already become suspicious, and by then the
// money is spent, the file is written or the link has been down for a
// minute. That is the whole argument for a strip that is always drawn,
// and it is why the strip carries these seven and not a logo.
//
// **It is also a single-authority repair, not only a nicer screen.**
// Before this file the seven were scattered: the model and the effort
// were live controls in the composer's second row, the link was the dot
// at the top of the rail, spending was two different pages with two
// different meanings, and autonomy was a setting on the settings page.
// Nothing could be compared with anything, and the two spending numbers
// were routinely read as the same number.
//
// **The two spending facts stay two cells.** One is what the run going
// on right now has cost and the other is what this city has cost
// altogether; they are different questions and a single figure answering
// both would be wrong for whichever one the reader had in mind. They
// come from one answer - `CostAnswer` carries `by_run` and `total` - so
// two cells here are still one question on the wire.
//
// **Nothing here is a control.** A cell is read, never pressed: the
// strip is 28px of note-sized text with no target in it, which is what
// lets it be this short. The model and the effort are drawn as text for
// a second reason as well - they are frozen once a session has begun,
// and a control that cannot be operated is worse than a word.

import { For, Show, createMemo } from "solid-js";

import { QUERIES } from "../core/asking";
import { buildingOf } from "../core/route";
import { usd } from "../core/time";
import type { Autonomy, CostAnswer, GovernanceAnswer } from "../wire";
import { useSay, useUi } from "../ui";

// How loudly a cell is drawn.
//
// Exhaustive, and it is the whole of the difference between the two
// kinds of fact on this strip: a setting is something the person chose
// and is here so they do not have to remember it, while a reading is
// something the city is doing to them and is here so they cannot miss
// it. `Alerting` is a reading whose current value is the one worth
// interrupting for - the link is down, the door is wide open, the work
// is not boxed in.
type Weight = "setting" | "reading" | "alerting";

const INK: Readonly<Record<Weight, string>> = {
  setting: "text-text-faint",
  reading: "text-text-quiet",
  alerting: "text-alert",
};

interface Cell {
  readonly key: string;
  // Already in the person's language.
  readonly label: string;
  readonly value: string;
  readonly weight: Weight;
}

// The dash a cell shows when the fact has no value yet. One spelling,
// so an unset model and an unspent city do not look like two different
// kinds of nothing.
const NOTHING = "—";

// Exhaustive over `Autonomy`, which is a literal on one arm and a
// struct on the other: the string arm is the owner, and anything else
// is somebody the owner named.
function autonomyWord(
  held: Autonomy | undefined,
  say: (key: "autonomy_owner" | "autonomy_clerk") => string,
): string {
  if (held === undefined) return NOTHING;
  return held === "owner" ? say("autonomy_owner") : say("autonomy_clerk");
}

export function Facts() {
  const ui = useUi();
  const say = useSay();

  // The run that is going. The strip speaks for one run because a
  // person watching a city is watching the thing that is moving; when
  // two are moving the newest is the one they just started.
  const live = createMemo(() =>
    Object.values(ui.conn.belief.runs)
      .filter((run) => run.doing.kind !== "frozen")
      .sort((a, b) => (a.started ?? 0) - (b.started ?? 0))
      .at(-1),
  );

  const endpoints = ui.conn.asking.ask(QUERIES.endpoints);
  const model = createMemo(() => {
    const held = endpoints();
    if (held === undefined || !("endpoints" in held)) return NOTHING;
    return held.endpoints.chosen.find((each) => each.tag === "main")?.model ?? say("talk_no_model");
  });

  const cost = ui.conn.asking.ask(QUERIES.cost);
  const spending = createMemo<CostAnswer | undefined>(() => {
    const held = cost();
    return held !== undefined && "cost" in held ? held.cost : undefined;
  });
  const thisRun = createMemo(() => {
    const run = live();
    const held = spending();
    if (run === undefined || held === undefined) return NOTHING;
    const found = held.by_run.find(([name]) => name === run.run);
    return found === undefined ? usd(0) : usd(found[1]);
  });
  const thisCity = createMemo(() => {
    const held = spending();
    return held === undefined ? NOTHING : usd(held.total);
  });

  const governance = ui.conn.asking.ask(QUERIES.governance);
  const autonomy = createMemo<GovernanceAnswer | undefined>(() => {
    const held = governance();
    return held !== undefined && "governance" in held ? held.governance : undefined;
  });

  // What boxes in the run that is going. A sandbox is a property of the
  // building the work happens in, so the question is asked of that
  // building rather than kept as a separate fact: one authority, read
  // where it lives.
  const boxed = createMemo(() => {
    const run = live();
    const addr = run?.addr;
    if (addr === null || addr === undefined) return undefined;
    return ui.conn.asking.ask({ building_view: { addr: buildingOf(addr) } });
  });
  const sandbox = createMemo<{ readonly value: string; readonly weight: Weight }>(() => {
    const asked = boxed();
    const held = asked?.();
    if (held === undefined || !("building" in held)) {
      return { value: NOTHING, weight: "reading" };
    }
    const limits = held.building.sandbox;
    if (limits === null || limits === undefined) {
      // Absence is a decision here, not a missing value: no layer named
      // a boundary, so the run has the machine. That is the reading
      // this cell exists to make impossible to miss.
      return { value: say("facts_sandbox_open"), weight: "alerting" };
    }
    return {
      value: say("facts_sandbox_mounts", { n: String(limits.mounts.length) }),
      weight: "reading",
    };
  });

  const link = () => ui.conn.state().kind;
  const linkWord = () => {
    switch (link()) {
      case "live":
        return say("link_live");
      case "refused":
        return say("link_refused");
      case "idle":
      case "opening":
      case "handshaking":
      case "backoff":
        return say("link_connecting");
    }
  };

  const cells = createMemo<Cell[]>(() => {
    const boxedIn = sandbox();
    return [
      { key: "model", label: say("facts_model"), value: model(), weight: "setting" },
      {
        key: "effort",
        label: say("talk_effort"),
        value: ui.effort() ?? NOTHING,
        weight: "setting",
      },
      { key: "run", label: say("facts_run"), value: thisRun(), weight: "reading" },
      { key: "city", label: say("facts_city"), value: thisCity(), weight: "reading" },
      {
        key: "link",
        label: say("facts_link"),
        value: linkWord(),
        weight: link() === "live" ? "reading" : "alerting",
      },
      {
        key: "autonomy",
        label: say("facts_autonomy"),
        value: autonomyWord(autonomy()?.autonomy, say),
        weight: "reading",
      },
      { key: "sandbox", label: say("facts_sandbox"), value: boxedIn.value, weight: boxedIn.weight },
    ];
  });

  return (
    // `role="status"` rather than a landmark: a screen reader is told
    // when one of these changes, and is not offered a region with seven
    // unlabelled numbers in it to navigate into.
    <footer
      class="flex h-facts shrink-0 items-center gap-wide overflow-x-auto border-t border-edge bg-chrome px-pane text-note whitespace-nowrap"
      aria-label={say("facts_region")}
      role="status"
    >
      <For each={cells()}>
        {(cell) => (
          // A reading worth interrupting for takes the one mark this
          // product uses for anything that needs a person: the bar on
          // the leading edge and a glyph, declared once in `theme.css`
          // as `asks`. Colour alone would be invisible in a
          // forced-colour mode, which is where it is needed most.
          <span
            class={`flex shrink-0 items-baseline gap-tight ${cell.weight === "alerting" ? "asks" : ""}`}
          >
            <Show when={cell.weight === "alerting"}>
              <span aria-hidden="true" class="self-center text-alert">
                !
              </span>
            </Show>
            <span class="text-text-disabled">{cell.label}</span>
            <span class={`figure ${INK[cell.weight]}`}>{cell.value}</span>
          </span>
        )}
      </For>
    </footer>
  );
}
