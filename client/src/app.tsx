// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The shell: one rail on the left, one content region, and the two
// things that may float over them - a refusal, and the palette. Which
// page shows is the address bar's decision, read on every `hashchange`.
// The keys: `g` then a letter goes somewhere, `[` opens the rail, `?`
// opens it to read the keys, Ctrl-K opens the palette.

import { Option } from "effect";
import { Match, Show, Switch, createEffect, createMemo, createSignal, onCleanup, onMount } from "solid-js";

import { paintMark } from "./core/mark";
import { DEFAULT_VIEW, MAYOR, current, toFragment } from "./core/route";
import type { View } from "./core/route";
import { useGo, useSay, useUi } from "./ui";
import { Building } from "./views/building";
import { City } from "./views/city";
import { Cost } from "./views/cost";
import { Palette } from "./views/palette";
import { Rail } from "./views/rail";
import { Record } from "./views/record";
import { Refusal } from "./views/refusal";
import { Run } from "./views/run";
import { Mcp } from "./views/mcp";
import { Setup } from "./views/setup";
import { Talk } from "./views/talk";
import { Gallery } from "./views/gallery";
import { Welcome } from "./views/welcome";

// Where `g` followed by a letter goes.
const GOES: Readonly<Record<string, View>> = {
  m: { kind: "talk", address: MAYOR },
  w: { kind: "talk", address: MAYOR },
  c: { kind: "city" },
  s: { kind: "setup" },
  x: { kind: "mcp" },
  r: { kind: "record", lens: "ledger" },
  $: { kind: "cost" },
};

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
  const [view, setView] = createSignal<View>(DEFAULT_VIEW);
  const [paletteOpen, setPaletteOpen] = createSignal(false);
  const [railOpen, setRailOpen] = createSignal(false);
  let prefix: string | null = null;

  const follow = () => {
    setView(Option.getOrElse(current(ui.bar), () => DEFAULT_VIEW));
  };
  const keys = (event: KeyboardEvent) => {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
      event.preventDefault();
      setPaletteOpen((open) => !open);
      return;
    }
    if (event.key === "Escape") {
      if (paletteOpen()) setPaletteOpen(false);
      else if (railOpen()) setRailOpen(false);
      return;
    }
    if (typing(event.target) || paletteOpen() || event.ctrlKey || event.metaKey || event.altKey) {
      prefix = null;
      return;
    }
    if (prefix === "g") {
      prefix = null;
      const to = GOES[event.key];
      if (to !== undefined) {
        event.preventDefault();
        go(to);
      }
      return;
    }
    if (event.key === "g") {
      prefix = "g";
    } else if (event.key === "[") {
      setRailOpen((open) => !open);
    } else if (event.key === "?") {
      setRailOpen(true);
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
  createEffect(() => {
    const name = ui.conn.belief.city ?? "sprawling";
    document.title = waiting() > 0 ? `(${String(waiting())}) ${name}` : name;
    document.documentElement.lang = ui.prefs.lang();
  });
  createEffect(() => {
    paintMark(document, waiting() > 0 ? "waiting" : working() ? "live" : "quiet");
  });

  return (
    <div class="flex h-screen bg-g0 font-sans text-body text-text">
      <Show when={view().kind !== "welcome"}>
        <Rail
          view={view()}
          open={railOpen()}
          onToggle={() => setRailOpen((open) => !open)}
          onPalette={() => setPaletteOpen(true)}
        />
      </Show>
      <main class="flex min-h-0 min-w-0 flex-1 flex-col overflow-y-auto" aria-label={say("region_main")}>
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
      <Refusal />
      <Show when={paletteOpen()}>
        <Palette onClose={() => setPaletteOpen(false)} />
      </Show>
      <span class="sr-only">{toFragment(view())}</span>
    </div>
  );
}
