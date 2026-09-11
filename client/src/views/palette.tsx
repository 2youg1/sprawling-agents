// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One box that reaches everything the bar does not show: every page,
// every building, every room with a run in it, the brake, the language,
// and - the moment a line begins with `/` - every verb this client
// understands. The verbs are not a list of their own: they come from
// `core/slash.ts`, which is the same table the composer's `/` menu
// reads, so a spelling learned in one place works in the other.

import { For, Show, createMemo, createSignal, onMount } from "solid-js";
import { Option } from "effect";

import { halt, release } from "../core/commands";
import { LANGS, endonym } from "../core/lang";
import { MAYOR, current, toFragment } from "../core/route";
import type { View } from "../core/route";
import { completed, offered } from "../core/slash";
import type { Reached, Slash, SlashHands } from "../core/slash";
import { useCommand, useGo, useSay, useUi } from "../ui";
import { Address } from "../core/address";
import type { RunBelief } from "../core/belief";

interface Entry {
  readonly label: string;
  readonly hint: string;
  readonly act: () => void;
}

function reached(run: RunBelief | undefined): Reached | null {
  return run === undefined ? null : { run: run.run, at: run.lastSeq };
}

export function Palette(props: { readonly onClose: () => void }) {
  const ui = useUi();
  const say = useSay();
  const go = useGo();
  const command = useCommand();
  const [query, setQuery] = createSignal("");
  const [cursor, setCursor] = createSignal(0);
  const [box, setBox] = createSignal<HTMLInputElement>();
  onMount(() => box()?.focus());

  const city = ui.conn.asking.ask("city_view");
  const endpoints = ui.conn.asking.ask("endpoint_view");
  const entries = createMemo<Entry[]>(() => {
    const goTo = (view: View, label: string, hint: string): Entry => ({
      label,
      hint,
      act: () => {
        go(view);
      },
    });
    const out: Entry[] = [
      goTo({ kind: "talk", address: MAYOR }, say("nav_mayor"), toFragment({ kind: "talk", address: MAYOR })),
      goTo({ kind: "city" }, say("nav_city"), "#/city"),
      goTo({ kind: "setup" }, say("nav_settings"), "#/setup"),
      goTo({ kind: "mcp" }, say("nav_mcp"), "#/mcp"),
      goTo({ kind: "record", lens: "ledger" }, say("rec_ledger"), "#/record"),
      goTo({ kind: "record", lens: "archive" }, say("rec_archive"), "#/record/archive"),
      goTo({ kind: "record", lens: "bin" }, say("rec_bin"), "#/record/bin"),
      goTo({ kind: "cost" }, say("cost_title"), "#/cost"),
      goTo({ kind: "welcome" }, say("setup_rerun"), "#/welcome"),
    ];
    const halted = ui.conn.belief.halted.includes("city");
    out.push({
      label: halted ? say("city_release") : say("city_stop"),
      hint: "halt",
      act: () => {
        command(halted ? release("city") : halt("city"));
      },
    });
    for (const lang of LANGS) {
      if (lang !== ui.prefs.lang()) {
        out.push({
          label: endonym(lang),
          hint: say("setup_language"),
          act: () => {
            ui.prefs.setLang(lang);
          },
        });
      }
    }
    const answer = city();
    if (answer !== undefined && "city" in answer) {
      for (const building of answer.city.buildings) {
        out.push(goTo({ kind: "building", address: building.addr }, building.addr, say("palette_building")));
      }
    }
    const rooms = new Set<string>();
    for (const run of Object.values(ui.conn.belief.runs)) {
      if (run.addr !== null && run.addr !== MAYOR) rooms.add(run.addr);
    }
    for (const room of rooms) {
      const address = Address.option(room);
      if (address._tag === "Some") {
        out.push(goTo({ kind: "talk", address: address.value }, room, say("palette_room")));
      }
    }
    return out;
  });

  // What a verb typed here may reach. The palette is not attached to a
  // room, so `here` is whatever room the address bar names and `live`
  // is the newest run still going anywhere: a person who types `/stop`
  // into this box means the one thing that is working.
  const models = createMemo(() => {
    const held = endpoints();
    if (held === undefined || !("endpoints" in held)) return [];
    return held.endpoints.endpoints.flatMap((endpoint) =>
      endpoint.models.map((model) => ({ endpoint: endpoint.name, model })),
    );
  });
  const here = createMemo(() => {
    const view = Option.getOrNull(current(ui.bar));
    return view !== null && view.kind === "talk" ? view.address : null;
  });
  const newest = (room: string): Reached | null =>
    reached(
      Object.values(ui.conn.belief.runs)
        .filter((run) => run.addr === room)
        .sort((a, b) => (b.started ?? 0) - (a.started ?? 0))
        .at(0),
    );
  const live = createMemo<Reached | null>(() =>
    reached(
      Object.values(ui.conn.belief.runs)
        .filter((run) => run.doing.kind !== "frozen")
        .sort((a, b) => (b.started ?? 0) - (a.started ?? 0))
        .at(0),
    ),
  );

  // A verb runs, and the box closes unless the verb put words back in
  // it - which is what `/help` does, and the one reason to stay open.
  const runSlash = (chosen: Slash) => {
    const written: { line: string | null } = { line: null };
    const hands: SlashHands = {
      command,
      go,
      here: here(),
      live: live(),
      newest,
      models: models(),
      effort: ui.prefs.effort(),
      setEffort: (effort) => {
        ui.prefs.setEffort(effort);
      },
      goal: say("talk_goal"),
      write: (line) => {
        written.line = line;
        setQuery(line);
        setCursor(0);
      },
    };
    const line = query().trim();
    const needs = chosen.grammar.startsWith("<");
    const cut = line.search(/\s/);
    const verb = cut < 0 ? line : line.slice(0, cut);
    const rest = cut < 0 ? "" : line.slice(cut + 1).trim();
    if (verb !== chosen.spelling || (needs && rest === "")) {
      setQuery(`${chosen.spelling} `);
      setCursor(0);
      return;
    }
    chosen.run(hands, { verb, words: rest === "" ? [] : rest.split(/\s+/), rest });
    if (written.line === null || written.line === "") {
      props.onClose();
    }
  };

  const commands = createMemo<Entry[]>(() =>
    offered(query().trim()).map((each) => ({
      label: each.grammar === "" ? each.spelling : `${each.spelling} ${each.grammar}`,
      hint: say(each.about),
      act: () => {
        runSlash(each);
      },
    })),
  );
  const shown = createMemo(() => {
    const needle = query().trim().toLowerCase();
    if (needle.startsWith("/")) {
      return commands();
    }
    const all = entries();
    return needle === ""
      ? all
      : all.filter((entry) => `${entry.label} ${entry.hint}`.toLowerCase().includes(needle));
  });
  const pick = (entry: Entry | undefined) => {
    if (entry === undefined) return;
    entry.act();
    // A verb decides for itself whether the box stays; everything else
    // is a place to go, and going there closes it.
    if (!query().trim().startsWith("/")) {
      props.onClose();
    }
  };

  return (
    <div
      class="fixed inset-0 z-20 flex items-start justify-center bg-g0/70 pt-section"
      onClick={() => {
        props.onClose();
      }}
    >
      <div
        class="w-full max-w-measure rounded-panel bg-g1 p-snug shadow-composer"
        role="dialog"
        aria-label={say("nav_palette")}
        onClick={(event) => {
          event.stopPropagation();
        }}
      >
        <input
          ref={setBox}
          class="w-full rounded-control bg-g2 px-base py-snug text-body outline-none placeholder:text-text-disabled"
          placeholder={say("palette_placeholder")}
          value={query()}
          onInput={(event) => {
            setQuery(event.currentTarget.value);
            setCursor(0);
          }}
          onKeyDown={(event) => {
            if (event.key === "Tab" && query().trim().startsWith("/")) {
              event.preventDefault();
              setQuery(completed(query().trim()));
              setCursor(0);
            } else if (event.key === "ArrowDown") {
              event.preventDefault();
              setCursor((at) => Math.min(at + 1, shown().length - 1));
            } else if (event.key === "ArrowUp") {
              event.preventDefault();
              setCursor((at) => Math.max(at - 1, 0));
            } else if (event.key === "Enter") {
              event.preventDefault();
              pick(shown()[cursor()]);
            }
          }}
        />
        <ul class="mt-snug max-h-palette overflow-y-auto">
          <For each={shown()}>
            {(entry, index) => (
              <li>
                <button
                  type="button"
                  class={`flex w-full items-center justify-between gap-snug rounded-control px-base py-snug text-left text-body hover:bg-g2 ${index() === cursor() ? "bg-g2" : ""}`}
                  onMouseEnter={() => setCursor(index())}
                  onClick={() => {
                    pick(entry);
                  }}
                >
                  <span class="truncate font-mono">{entry.label}</span>
                  <span class="shrink-0 text-note text-text-disabled">{entry.hint}</span>
                </button>
              </li>
            )}
          </For>
        </ul>
        <Show when={shown().length === 0}>
          <p class="px-base py-snug text-note text-text-disabled">{say("rec_nothing")}</p>
        </Show>
      </div>
    </div>
  );
}
