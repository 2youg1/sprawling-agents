<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The strip along the bottom of every page, and the one home of the
  // seven facts a person needs while something is being done on their
  // behalf (client-SPEC 7D): model · effort · this run's cost · the
  // city's cost · connection · gate · sandbox.
  //
  // **Four of the seven are risk readings** - what this has cost, how
  // far the door is open, whether the work is boxed in, and whether the
  // page is still hearing from the city. A risk reading has to be
  // visible when nobody is looking for it: behind a page somebody has
  // to open, it only works after they have already become suspicious.
  //
  // **The two spending facts stay two cells.** One is what the run going
  // on right now has cost and the other is what this city has cost
  // altogether; they come from one answer - `CostAnswer` carries
  // `by_run` and `total` - so two cells here are still one question on
  // the wire.
  //
  // **The core cell watches only the summary.** The strip is on every
  // page, so it asks the city for the cheapest reading there is - its
  // own process, about a microsecond a second - and never opens the
  // machine-wide counters the monitor panel pays for (sprawling-SPEC
  // 8-90).
  //
  // **Nothing here is a control.** A cell is read, never pressed: the
  // strip is 28px of note-sized text with no target in it, which is
  // what lets it be this short.

  import { derived } from "svelte/store";
  import type { Readable } from "svelte/store";

  import { QUERIES } from "../core/asking";
  import type { RunBelief } from "../core/belief";
  import { fill, say } from "../core/lang";
  import { summary } from "../core/monitor";
  import { buildingOf } from "../core/route";
  import { usd } from "../core/time";
  import { ui } from "../ui";
  import type { Address, Answer, Autonomy, CostAnswer, GovernanceAnswer } from "../wire";

  // How loudly a cell is drawn. Exhaustive, and it is the whole of the
  // difference between the two kinds of fact on this strip: a setting
  // is something the person chose and is here so they do not have to
  // remember it, while a reading is something the city is doing to them
  // and is here so they cannot miss it. `alerting` is a reading whose
  // current value is the one worth interrupting for.
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
    // A cell that changes every second is left out of what the strip's
    // `role="status"` announces, or a screen reader would say it once
    // a second.
    readonly announced?: "silently";
  }

  // The dash a cell shows when the fact has no value yet. One spelling,
  // so an unset model and an unspent city do not look like two
  // different kinds of nothing.
  const NOTHING = "—";

  const u = ui();
  const lang = u.lang;
  const belief = u.conn.belief;
  const link = u.conn.state;
  const effort = u.effort;
  const endpoints = u.conn.asking.ask(QUERIES.endpoints);
  const cost = u.conn.asking.ask(QUERIES.cost);
  const governance = u.conn.asking.ask(QUERIES.governance);
  const samples = u.conn.monitor.samples;

  $effect(() => u.conn.monitor.watchSummary());

  // The run that is going. The strip speaks for one run because a
  // person watching a city is watching the thing that is moving; when
  // two are moving the newest is the one they just started.
  const moving = derived(belief, (held): RunBelief | null => {
    const going = Object.values(held.runs).filter((run) => run.doing.kind !== "frozen");
    going.sort((a, b) => (a.started ?? 0) - (b.started ?? 0));
    return going.at(-1) ?? null;
  });

  // What boxes in the run that is going. A sandbox is a property of the
  // building the work happens in, so the question is asked of that
  // building rather than kept as a separate fact: one authority, read
  // where it lives. The address is the thing kept stable here, so a
  // belief write that only replaces the run's own object leaves the
  // question alone.
  const boxed = derived<Readable<Address | null>, Answer | undefined>(
    derived(moving, (run) => {
      const at = run?.addr ?? null;
      return at === null ? null : buildingOf(at);
    }),
    (addr, set) => {
      if (addr === null) {
        set(undefined);
        return;
      }
      return u.conn.asking.ask({ building_view: { addr } }).subscribe(set);
    },
  );

  // Exhaustive over `Autonomy`, which is a literal on one arm and a
  // struct on the other: the string arm is the owner, and anything
  // else is somebody the owner named.
  function autonomyWord(held: Autonomy | undefined): string {
    if (held === undefined) return NOTHING;
    return held === "owner" ? say($lang, "autonomy_owner") : say($lang, "autonomy_clerk");
  }

  function linkWord(): string {
    switch ($link.kind) {
      case "live":
        return say($lang, "link_live");
      case "refused":
        return say($lang, "link_refused");
      case "idle":
      case "opening":
      case "handshaking":
      case "backoff":
        return say($lang, "link_connecting");
    }
  }

  const model = $derived.by((): string => {
    const held = $endpoints;
    if (held === undefined || !("endpoints" in held)) return NOTHING;
    return (
      held.endpoints.chosen.find((each) => each.tag === "main")?.model ?? say($lang, "talk_no_model")
    );
  });

  const spending = $derived.by((): CostAnswer | undefined => {
    const held = $cost;
    return held !== undefined && "cost" in held ? held.cost : undefined;
  });

  const thisRun = $derived.by((): string => {
    const run = $moving;
    const held = spending;
    if (run === null || held === undefined) return NOTHING;
    const found = held.by_run.find(([name]) => name === run.run);
    return found === undefined ? usd(0) : usd(found[1]);
  });

  const thisCity = $derived.by((): string => {
    const held = spending;
    return held === undefined ? NOTHING : usd(held.total);
  });

  const autonomy = $derived.by((): GovernanceAnswer | undefined => {
    const held = $governance;
    return held !== undefined && "governance" in held ? held.governance : undefined;
  });

  const sandbox = $derived.by((): { readonly value: string; readonly weight: Weight } => {
    const held = $boxed;
    if (held === undefined || !("building" in held)) {
      return { value: NOTHING, weight: "reading" };
    }
    const limits = held.building.sandbox;
    if (limits === null || limits === undefined) {
      // Absence is a decision here, not a missing value: no layer named
      // a boundary, so the run has the machine. That is the reading
      // this cell exists to make impossible to miss.
      return { value: say($lang, "facts_sandbox_open"), weight: "alerting" };
    }
    return {
      value: fill(say($lang, "facts_sandbox_mounts"), { n: String(limits.mounts.length) }),
      weight: "reading",
    };
  });

  const core = $derived.by((): string => {
    const latest = $samples.at(-1);
    return latest === undefined ? NOTHING : fill(say($lang, "facts_core_reading"), summary(latest));
  });

  const cells = $derived.by((): Cell[] => {
    const boxedIn = sandbox;
    return [
      { key: "model", label: say($lang, "facts_model"), value: model, weight: "setting" },
      {
        key: "effort",
        label: say($lang, "talk_effort"),
        value: $effort ?? NOTHING,
        weight: "setting",
      },
      { key: "run", label: say($lang, "facts_run"), value: thisRun, weight: "reading" },
      { key: "city", label: say($lang, "facts_city"), value: thisCity, weight: "reading" },
      {
        key: "link",
        label: say($lang, "facts_link"),
        value: linkWord(),
        weight: $link.kind === "live" ? "reading" : "alerting",
      },
      {
        key: "autonomy",
        label: say($lang, "facts_autonomy"),
        value: autonomyWord(autonomy?.autonomy),
        weight: "reading",
      },
      { key: "sandbox", label: say($lang, "facts_sandbox"), value: boxedIn.value, weight: boxedIn.weight },
      {
        key: "core",
        label: say($lang, "facts_core"),
        value: core,
        weight: "reading",
        announced: "silently",
      },
    ];
  });
</script>

<!-- `role="status"` rather than a landmark: a screen reader is told
when one of these changes, and is not offered a region with seven
unlabelled numbers in it to navigate into. -->
<footer
  class="flex h-facts shrink-0 items-center gap-wide overflow-x-auto border-t border-edge bg-chrome px-pane text-note whitespace-nowrap"
  aria-label={say($lang, "facts_region")}
  role="status"
>
  {#each cells as cell (cell.key)}
    <!-- A reading worth interrupting for takes the one mark this
    product uses for anything that needs a person: the bar on the
    leading edge and a glyph, declared once in `theme.css` as `asks`.
    Colour alone would be invisible in a forced-colour mode, which is
    where it is needed most. -->
    <span
      class={["flex shrink-0 items-baseline gap-tight", cell.weight === "alerting" ? "asks" : ""]}
      aria-live={cell.announced === "silently" ? "off" : undefined}
    >
      {#if cell.weight === "alerting"}
        <span aria-hidden="true" class="self-center text-alert">!</span>
      {/if}
      <span class="text-text-disabled">{cell.label}</span>
      <span class={["figure", INK[cell.weight]]}>{cell.value}</span>
    </span>
  {/each}
</footer>
