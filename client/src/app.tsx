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
// A prefix key arms for `PREFIX_MS` and says so on the rail while it
// waits, so `g` followed by nothing teaches rather than swallows.
//
// A stopped city is said once, here, as a banner over every page: the
// city page used to draw a crescent nobody could read and dim itself to
// 40%, which is a mood rather than a message.

import { Option } from "effect";
import { Match, Show, Switch, createEffect, createMemo, createSignal, onCleanup, onMount } from "solid-js";

import { halt, release } from "./core/commands";
import { PREFIX_MS, keymap } from "./core/keys";
import type { Action } from "./core/keys";
import { paintMark } from "./core/mark";
import { DEFAULT_VIEW, MAYOR, current, toFragment } from "./core/route";
import type { View } from "./core/route";
import { useCommand, useGo, useSay, useUi } from "./ui";
import { Cheatsheet } from "./views/parts/kbd";
import { Building } from "./views/building";
import { City } from "./views/city";
import { Cost } from "./views/cost";
import { Palette } from "./views/palette";
import { Rail } from "./views/rail";
import { Record } from "./views/record";
import { Notices } from "./views/notices";
import { Refusal } from "./views/refusal";
import { Run } from "./views/run";
import { Mcp } from "./views/mcp";
import { Setup } from "./views/setup";
import { Talk } from "./views/talk";
import { Gallery } from "./views/gallery";
import { Welcome } from "./views/welcome";

// Where each `go.*` action lands. Every other action moves the shell
// rather than the address bar, and is answered below.
const GOES: Readonly<Record<string, View>> = {
  "go.talk": { kind: "talk", address: MAYOR },
  "go.waiting": { kind: "talk", address: MAYOR },
  "go.city": { kind: "city" },
  "go.setup": { kind: "setup" },
  "go.mcp": { kind: "mcp" },
  "go.record": { kind: "record", lens: "ledger" },
  "go.cost": { kind: "cost" },
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
  const [railOpen, setRailOpen] = createSignal(false);
  const [prefix, setPrefix] = createSignal<string | null>(null);
  let forget: ReturnType<typeof setTimeout> | undefined;
  // What opened the sheet, so closing it puts the focus back where the
  // person left it.
  let opener: HTMLElement | null = null;

  const follow = () => {
    setView(Option.getOrElse(current(ui.bar), () => DEFAULT_VIEW));
  };
  const drop = () => {
    clearTimeout(forget);
    setPrefix(null);
  };
  const arm = (key: string) => {
    clearTimeout(forget);
    setPrefix(key);
    forget = setTimeout(() => {
      setPrefix(null);
    }, PREFIX_MS);
  };
  const closeSheet = () => {
    setSheetOpen(false);
    opener?.focus();
    opener = null;
  };
  const act = (action: Action) => {
    const to = GOES[action];
    if (to !== undefined) {
      go(to);
      return;
    }
    switch (action) {
      case "palette":
        setPaletteOpen((open) => !open);
        return;
      case "rail.toggle":
        setRailOpen((open) => !open);
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
      // Every `go.*` action was answered by the table above; naming them
      // keeps a new one a decision here rather than a silence.
      case "go.talk":
      case "go.city":
      case "go.mcp":
      case "go.record":
      case "go.cost":
      case "go.setup":
      case "go.waiting":
        return;
    }
  };
  const keys = (event: KeyboardEvent) => {
    const accel = event.ctrlKey || event.metaKey;
    if (event.key === "Escape") {
      drop();
      if (paletteOpen()) setPaletteOpen(false);
      else if (sheetOpen()) closeSheet();
      else if (railOpen()) setRailOpen(false);
      return;
    }
    // Inside a text box and inside the palette, only a chord that holds
    // the accelerator is the shell's; everything else is being typed.
    if ((typing(event.target) || paletteOpen()) && !accel) {
      drop();
      return;
    }
    const action = bindings.acting(event, prefix());
    if (action !== null) {
      event.preventDefault();
      drop();
      act(action);
      return;
    }
    if (prefix() !== null) {
      drop();
      return;
    }
    if (!accel && !event.altKey && bindings.prefixes().includes(event.key)) {
      event.preventDefault();
      arm(event.key);
    }
  };
  onMount(() => {
    follow();
    window.addEventListener("hashchange", follow);
    window.addEventListener("keydown", keys);
  });
  onCleanup(() => {
    clearTimeout(forget);
    window.removeEventListener("hashchange", follow);
    window.removeEventListener("keydown", keys);
  });

  // The run list is the ground every page stands on: asked here so it
  // is always watched, and folded into what the page believes.
  ui.conn.asking.ask("city_view");

  // Whether this city can take a dispatch at all: a `main` model is
  // chosen. Until then the first page is the welcome, unless the person
  // has already walked it and asked to be left alone.
  const endpoints = ui.conn.asking.ask("endpoint_view");
  const ready = createMemo(() => {
    const answer = endpoints();
    if (answer === undefined || !("endpoints" in answer)) {
      return undefined;
    }
    return answer.endpoints.chosen.some((chosen) => chosen.tag === "main");
  });
  createEffect(() => {
    if (ready() === false && !ui.prefs.welcomed() && view().kind === "talk") {
      go({ kind: "welcome" });
    }
  });

  // The document title and the tab's icon carry what a hidden tab most
  // needs to say: how many things wait for the person, and whether the
  // city is working at all.
  const approvals = ui.conn.asking.ask("approval_queue");
  const waiting = createMemo(() => {
    const answer = approvals();
    return answer !== undefined && "approvals" in answer ? answer.approvals.items.length : 0;
  });
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
    document.documentElement.lang = ui.prefs.lang();
  });
  createEffect(() => {
    paintMark(document, waiting() > 0 ? "waiting" : working() ? "live" : "quiet");
  });

  return (
    <div class="relative flex h-screen bg-g0 font-sans text-body text-text">
      <a
        href="#main"
        class="sr-only focus:not-sr-only focus:absolute focus:top-snug focus:left-snug focus:z-30 focus:rounded-control focus:bg-g2 focus:px-base focus:py-snug focus:text-label focus:text-text"
      >
        {say("skip_main")}
      </a>
      <Show when={view().kind !== "welcome"}>
        <Rail
          view={view()}
          open={railOpen()}
          prefix={prefix()}
          onToggle={() => setRailOpen((open) => !open)}
          onPalette={() => setPaletteOpen(true)}
        />
      </Show>
      <div class="flex min-h-0 min-w-0 flex-1 flex-col">
        <Show when={halted()}>
          <div
            class="drop flex shrink-0 flex-wrap items-center gap-base border-b border-g2 bg-g1 px-pane py-snug text-label"
            role="status"
          >
            <span class="inline-block size-dot shrink-0 rounded-pill bg-alert" />
            <span class="text-text">{say("halt_title")}</span>
            <Show when={frozen() > 0}>
              <span class="text-text-quiet">{say("halt_frozen", { n: String(frozen()) })}</span>
            </Show>
            <button
              type="button"
              class="ml-auto rounded-control px-base py-tight font-mono text-label text-accent hover:bg-g2"
              onClick={() => command(release("city"))}
            >
              {say("city_release")}
            </button>
          </div>
        </Show>
        <div class="pointer-events-none absolute top-snug right-pane z-20 flex justify-end">
          <div class="pointer-events-auto">
            <Notices />
          </div>
        </div>
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
          <Match when={view().kind === "welcome"}>
            <Welcome />
          </Match>
          <Match when={view().kind === "gallery"}>
            <Gallery />
          </Match>
          </Switch>
        </main>
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
