// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The shell: one rail on the left, one content region, and the three
// things that may float over them - a refusal, the palette, and the
// sheet of keys. Which page shows is the address bar's decision, read
// on every `hashchange`.
//
// The keys are not spelled here. `core/keys` holds the action, the
// chord that reaches it and the person's own chord if they set one;
// this file asks it which action a press was and does that one thing.
// A stopped city is said once, here, as a banner over every page: the
// city page used to draw a crescent nobody could read and dim itself to
// 40%, which is a mood rather than a message.
//
// What the city has to tell the person is no longer a bell floating
// over the top right corner; it is the dot at the top of the rail
// (`views/notices.tsx`), which the rail mounts and this file does not
// have to place.

import { Option } from "effect";
import { Match, Show, Switch, createEffect, createMemo, createSignal, onCleanup, onMount } from "solid-js";

import { QUERIES } from "./core/asking";
import { halt, release } from "./core/commands";
import { keymap } from "./core/keys";
import type { Action } from "./core/keys";
import { paintMark } from "./core/mark";
import { RAILS } from "./core/prefs";
import { DEFAULT_VIEW, MAYOR, current, toFragment } from "./core/route";
import type { View } from "./core/route";
import { useApprovals, useCommand, useGo, useSay, useUi } from "./ui";
import { Cheatsheet } from "./views/parts/kbd";
import { Building } from "./views/building";
import { City } from "./views/city";
import { Cost } from "./views/cost";
import { Facts } from "./views/facts";
import { Palette } from "./views/palette";
import { Registry } from "./views/registry";
import { Rail } from "./views/rail";
import { Record } from "./views/record";
import { motionOff } from "./views/shared/motion";
import { Refusal } from "./views/refusal";
import { Run } from "./views/run";
import { Mcp } from "./views/mcp";
import { Setup } from "./views/setup";
import { Talk } from "./views/talk";
import { Gallery } from "./views/gallery";
import { Welcome } from "./views/welcome";

// The actions that move the address bar. Derived from `Action` rather
// than written out, so an action named `go.*` in `core/keys` has to
// land somewhere here before this file compiles.
type GoAction = Extract<Action, `go.${string}`>;

// Where each of them lands. Every other action moves the shell rather
// than the address bar, and is answered by the switch below.
//
// **Keyed by `GoAction`, which is what makes this table checked**: a
// key spelled wrong is not an action, and an action left out is a
// missing property. Before that it was keyed by `string`, so the name
// of a destination had two homes - this table and `ACTIONS` - and
// neither could tell the other was wrong (roadmap B-77).
const GOES: Readonly<Record<GoAction, View>> = {
  "go.talk": { kind: "talk", address: MAYOR },
  "go.waiting": { kind: "talk", address: MAYOR },
  "go.city": { kind: "city" },
  "go.setup": { kind: "setup" },
  "go.mcp": { kind: "mcp" },
  "go.record": { kind: "record", lens: "ledger" },
  "go.cost": { kind: "cost" },
  "go.registry": { kind: "registry" },
};

// The box a person writes in, wherever the page put it. Reached by the
// element it is rather than by a name this file would have to keep in
// step with the composer.
function focusComposer(): void {
  const box = document.querySelector("main textarea");
  if (box instanceof HTMLTextAreaElement) {
    box.focus();
  }
}

function typing(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    target instanceof HTMLSelectElement ||
    (target instanceof HTMLElement && target.isContentEditable)
  );
}

export function App() {
  const ui = useUi();
  const say = useSay();
  const go = useGo();
  const command = useCommand();
  const bindings = keymap();
  const [view, setView] = createSignal<View>(DEFAULT_VIEW);
  const [paletteOpen, setPaletteOpen] = createSignal(false);
  const [sheetOpen, setSheetOpen] = createSignal(false);
  // The rail's posture is the person's, kept where their other
  // postures are kept: it used to be a signal in this function, so a
  // reload put the column back and somebody who works with it away
  // put it away again every morning. Three postures, cycled by one
  // chord in the order `RAILS` states.
  const railPosture = () => ui.prefs.held().rail;
  const cycleRail = () => {
    const at = RAILS.indexOf(railPosture());
    ui.prefs.setRail(RAILS[(at + 1) % RAILS.length] ?? "glyphs");
  };
  // What opened the sheet, so closing it puts the focus back where the
  // person left it.
  let opener: HTMLElement | null = null;

  // One page replaces another. A swap is a cut; a view transition
  // carries the rail's current item and the page's title across, so
  // the eye keeps the thing it was already looking at.
  //
  // It wraps the read of the address bar rather than the write of it,
  // because `hashchange` arrives in a later task: a transition around
  // `go()` would finish before the page had changed. Doing it here
  // also covers the back button and every `<a href="#/…">` on the
  // page, which `go()` never sees.
  //
  // Firefox has not shipped the API, and a person who turned movement
  // off asked for no travel; both take the swap that was here before
  // (client-SPEC 9.0, rows 4 and 8).
  const follow = () => {
    const settle = () => {
      setView(Option.getOrElse(current(ui.bar), () => DEFAULT_VIEW));
    };
    if (!("startViewTransition" in document) || motionOff(document.documentElement)) {
      settle();
      return;
    }
    document.startViewTransition(settle);
  };
  const closeSheet = () => {
    setSheetOpen(false);
    opener?.focus();
    opener = null;
  };
  const act = (action: Action) => {
    switch (action) {
      // The eight arms that share a body are exactly `GoAction`, so
      // the lookup is checked here rather than guarded at run time.
      case "go.talk":
      case "go.city":
      case "go.mcp":
      case "go.record":
      case "go.cost":
      case "go.registry":
      case "go.setup":
      case "go.waiting":
        go(GOES[action]);
        return;
      case "palette":
        setPaletteOpen((open) => !open);
        return;
      case "rail.toggle":
        cycleRail();
        return;
      case "help":
        opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
        setSheetOpen(true);
        return;
      case "composer.focus":
        focusComposer();
        return;
      case "run.stop":
        command(halt("city"));
        return;
    }
  };
  const keys = (event: KeyboardEvent) => {
    const accel = event.ctrlKey || event.metaKey;
    if (event.key === "Escape") {
      if (paletteOpen()) setPaletteOpen(false);
      else if (sheetOpen()) closeSheet();
      else if (railPosture() === "named") ui.prefs.setRail("glyphs");
      return;
    }
    // Inside a text box and inside the palette, only a chord that holds
    // the accelerator is the shell's; everything else is being typed.
    if ((typing(event.target) || paletteOpen()) && !accel) {
      return;
    }
    const action = bindings.acting(event);
    if (action !== null) {
      event.preventDefault();
      act(action);
    }
  };
  onMount(() => {
    follow();
    window.addEventListener("hashchange", follow);
    window.addEventListener("keydown", keys);
  });
  onCleanup(() => {
    window.removeEventListener("hashchange", follow);
    window.removeEventListener("keydown", keys);
  });

  // The run list is the ground every page stands on: asked here so it
  // is always watched, and folded into what the page believes.
  ui.conn.asking.ask(QUERIES.city);

  // Whether this city can take a dispatch at all: a `main` model is
  // chosen. Until then the first page is the welcome, unless the person
  // has already walked it and asked to be left alone.
  const endpoints = ui.conn.asking.ask(QUERIES.endpoints);
  const ready = createMemo(() => {
    const answer = endpoints();
    if (answer === undefined || !("endpoints" in answer)) {
      return undefined;
    }
    return answer.endpoints.chosen.some((chosen) => chosen.tag === "main");
  });
  createEffect(() => {
    if (ready() === false && !ui.prefs.held().welcomed && view().kind === "talk") {
      go({ kind: "welcome" });
    }
  });

  // The document title and the tab's icon carry what a hidden tab most
  // needs to say: how many things wait for the person, and whether the
  // city is working at all.
  const approvals = useApprovals();
  const waiting = () => approvals().length;
  const working = createMemo(() => Object.values(ui.conn.belief.runs).some((run) => run.doing.kind !== "frozen"));
  const halted = () => ui.conn.belief.halted.includes("city");
  // How many runs this city cancelled. The wire carries no count of
  // what one halt froze, so this counts the runs whose own freeze says
  // `cancelled`, which is what a halt writes.
  const frozen = createMemo(
    () =>
      Object.values(ui.conn.belief.runs).filter(
        (run) => run.doing.kind === "frozen" && run.doing.completion === "cancelled",
      ).length,
  );
  createEffect(() => {
    const name = ui.conn.belief.city ?? "sprawling";
    document.title = waiting() > 0 ? `(${String(waiting())}) ${name}` : name;
    document.documentElement.lang = ui.prefs.held().lang;
  });
  createEffect(() => {
    paintMark(document, waiting() > 0 ? "waiting" : working() ? "live" : "quiet");
  });

  return (
    <div class="relative flex h-screen bg-page font-sans text-body text-text">
      <a
        href="#main"
        class="sr-only focus:not-sr-only focus:absolute focus:top-snug focus:left-snug focus:z-30 focus:rounded-control focus:bg-raised focus:px-base focus:py-snug focus:text-label focus:text-text"
      >
        {say("skip_main")}
      </a>
      <Show when={view().kind !== "welcome"}>
        <Rail
          view={view()}
          posture={railPosture()}
          onToggle={cycleRail}
          onPalette={() => setPaletteOpen(true)}
        />
      </Show>
      <div class="flex min-h-0 min-w-0 flex-1 flex-col">
        <Show when={halted()}>
          <div
            class="drop flex shrink-0 flex-wrap items-center gap-base border-b border-edge bg-chrome px-pane py-snug text-label"
            role="status"
          >
            <span class="inline-block size-dot shrink-0 rounded-pill bg-alert" />
            <span class="text-text">{say("halt_title")}</span>
            <Show when={frozen() > 0}>
              <span class="text-text-quiet">{say("halt_frozen", { n: String(frozen()) })}</span>
            </Show>
            <button
              type="button"
              class="ml-auto rounded-control px-base py-tight font-mono text-label text-accent hover:bg-raised"
              onClick={() => command(release("city"))}
            >
              {say("city_release")}
            </button>
          </div>
        </Show>
        <main id="main" class="flex min-h-0 min-w-0 flex-1 flex-col overflow-y-auto" aria-label={say("region_main")}>
          <Switch>
          <Match keyed when={view().kind === "talk" ? view() : undefined}>
            {(talk) => (talk.kind === "talk" ? <Talk address={talk.address} /> : null)}
          </Match>
          <Match when={view().kind === "city"}>
            <City />
          </Match>
          <Match keyed when={view().kind === "building" ? view() : undefined}>
            {(building) => (building.kind === "building" ? <Building address={building.address} /> : null)}
          </Match>
          <Match keyed when={view().kind === "run" ? view() : undefined}>
            {(run) => (run.kind === "run" ? <Run run={run.run} /> : null)}
          </Match>
          <Match when={view().kind === "setup"}>
            <Setup />
          </Match>
          <Match when={view().kind === "mcp"}>
            <Mcp />
          </Match>
          <Match keyed when={view().kind === "record" ? view() : undefined}>
            {(record) => (record.kind === "record" ? <Record lens={record.lens} /> : null)}
          </Match>
          <Match when={view().kind === "cost"}>
            <Cost />
          </Match>
          <Match when={view().kind === "registry"}>
            <Registry />
          </Match>
          <Match when={view().kind === "welcome"}>
            <Welcome />
          </Match>
          <Match when={view().kind === "gallery"}>
            <Gallery />
          </Match>
          </Switch>
        </main>
        {/* Under every page, and outside `<main>` on purpose: it is a
            control surface rather than content, so it does not print,
            and a person tabbing through the page does not walk into
            seven unlabelled readings. The welcome walk is the one
            screen without it - nothing is running yet, and a strip of
            dashes would teach a first-time reader that this product
            shows them nothing. */}
        <Show when={view().kind !== "welcome"}>
          <Facts />
        </Show>
      </div>
      <Refusal />
      <Show when={paletteOpen()}>
        <Palette onClose={() => setPaletteOpen(false)} />
      </Show>
      <Show when={sheetOpen()}>
        <Cheatsheet onClose={closeSheet} />
      </Show>
      <span class="sr-only">{toFragment(view())}</span>
    </div>
  );
}
