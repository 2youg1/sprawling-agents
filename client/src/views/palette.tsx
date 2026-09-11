// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One box that reaches everything the bar does not show: every page,
// every building, every room with a run in it, the brake, and the
// language. Typed into rather than browsed, so it costs the page
// nothing until it is asked for.

import { For, createMemo, createSignal, onMount } from "solid-js";

import { TEMPLATES, createBuilding, fork, halt, release } from "../core/commands";
import type { Template } from "../core/commands";
import { LANGS, endonym } from "../core/lang";
import { MAYOR, toFragment } from "../core/route";
import type { View } from "../core/route";
import { useCommand, useGo, useSay, useUi } from "../ui";
import { Address } from "../core/address";

interface Entry {
  readonly label: string;
  readonly hint: string;
  readonly act: () => void;
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
  // A verb typed with its argument. The box is already a text box, so a
  // verb that needs a name takes it here instead of growing a form of its
  // own somewhere else - which is also why neither of these two appears
  // as a button on a page that has decided how many controls it has.
  const typed = createMemo<Entry[]>(() => {
    const words = query().trim().split(/\s+/);
    const verb = words.at(0);
    const subject = words.at(1);
    if (verb === undefined || subject === undefined || subject === "") return [];
    if (verb === "raise") {
      const addr = Address.option(subject);
      const asked = words.at(2) ?? "minimal";
      const template = TEMPLATES.find((known): known is Template => known === asked);
      if (addr._tag === "None" || template === undefined) return [];
      const at = addr.value;
      return [
        {
          label: say("palette_raise", { addr: subject }),
          hint: `raise ${template}`,
          act: () => {
            command(createBuilding(at, template));
          },
        },
      ];
    }
    if (verb === "fork") {
      const newest = Object.values(ui.conn.belief.runs)
        .filter((run) => run.addr === subject)
        .sort((a, b) => (b.started ?? 0) - (a.started ?? 0))
        .at(0);
      if (newest === undefined) return [];
      return [
        {
          label: say("palette_fork", { addr: subject }),
          hint: `fork ${String(newest.lastSeq)}`,
          act: () => {
            command(fork(newest.run, newest.lastSeq, null));
          },
        },
      ];
    }
    return [];
  });
  const shown = createMemo(() => {
    const needle = query().trim().toLowerCase();
    const all = entries();
    const matched =
      needle === ""
        ? all
        : all.filter((entry) => `${entry.label} ${entry.hint}`.toLowerCase().includes(needle));
    return [...typed(), ...matched];
  });
  const pick = (entry: Entry | undefined) => {
    if (entry === undefined) return;
    entry.act();
    props.onClose();
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
            if (event.key === "ArrowDown") {
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
                  class={`flex w-full items-center justify-between rounded-control px-base py-snug text-left text-body hover:bg-g2 ${index() === cursor() ? "bg-g2" : ""}`}
                  onMouseEnter={() => setCursor(index())}
                  onClick={() => {
                    pick(entry);
                  }}
                >
                  <span>{entry.label}</span>
                  <span class="font-mono text-note text-text-disabled">{entry.hint}</span>
                </button>
              </li>
            )}
          </For>
        </ul>
      </div>
    </div>
  );
}
