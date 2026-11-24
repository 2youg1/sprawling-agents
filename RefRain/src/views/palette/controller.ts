// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal, onSettled, untrack } from "solid-js";
import { storeValue } from "../../core/solid_store";
import { Option, Schema } from "effect";
import { heldIn } from "../../core/belief/rooms";
import { newestWorking } from "../../core/belief/live";
import { runInFront } from "../../core/in_front";

import { QUERIES } from "../../core/asking";
import { halt, release } from "../../core/commands";
import { LABELS } from "../../core/keys";
import type { Action } from "../../core/keys";
import { canRecord } from "../../core/speaking";
import { LANGS, endonym, say } from "../../core/lang";
import { MAYOR, current, toFragment } from "../../core/route";
import type { View } from "../../core/route";
import { cityIsShut, CITY } from "../../core/scope";
import { RELEASE_ALL, offered } from "../../core/slash";
import { SECTIONS, reached } from "../../core/slash_hands";
import type { Reached, Slash, SlashHands } from "../../core/slash_hands";
import { ui } from "../../ui";
import { Address } from "../../wire";
import { carryPlaces, directEntry, SESSIONS_AT_REST, sessionPlaces, whyFor } from "../palette";
import type { Entry, Group } from "./entry";
import { openFinder } from "../finding";
import { noteOf, waitWhy } from "../talk/branch_note";
import { branchFrom, branchInto } from "../talk/branching";
import { askToSpeak } from "../talk/speak_asked";
import { sessionHands } from "../talk/session_hands";

export interface PaletteInput {
  readonly here?: Address | undefined;
  readonly initial?: string | undefined;
  readonly view?: View | null | undefined;
  readonly onClose: () => void;
}

export function paletteController(props: PaletteInput) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const held = storeValue(u.prefs.held);
  const effort = storeValue(u.effort);
  const policy = storeValue(u.policy);
  const branching = storeValue(u.branches.held);
  const [query, setQuery] = createSignal(untrack(() => props.initial ?? ""));
  const [cursor, setCursor] = createSignal(0);
  const [carrying, setCarrying] = createSignal(false);
  const city = storeValue(u.conn.asking.ask(QUERIES.city));
  const endpoints = storeValue(u.conn.asking.ask(QUERIES.endpoints));
  const buildings = createMemo(() => {
    const answer = city();
    return answer !== undefined && "city" in answer ? answer.city.buildings : undefined;
  });
  const halted = createMemo(() => cityIsShut(belief().halted));
  const [page, setPage] = createSignal(Option.getOrNull(current(u.bar)));
  const viewed = createMemo(() => (props.view === undefined ? page() : props.view));
  onSettled(() => {
    const move = (): void => {
      setPage(Option.getOrNull(current(u.bar)));
    };
    window.addEventListener("hashchange", move);
    return () => window.removeEventListener("hashchange", move);
  });
  const here = createMemo(() => {
    const at = viewed();
    return props.here ?? (at?.kind === "talk" ? at.address : null);
  });
  const history = storeValue(() => {
    const room = here();
    return room === null ? undefined : u.conn.asking.ask({ sessions: { room } });
  });
  const sessionLines = createMemo(() => {
    const answer = history();
    return answer !== undefined && "sessions" in answer ? answer.sessions.sessions : [];
  });
  const live = createMemo(() => {
    const at = viewed();
    return reached(at === null ? undefined : runInFront(belief(), at));
  });
  const carryFrom = createMemo(() => {
    const room = here();
    const at = viewed();
    return room !== null &&
      at?.kind === "talk" &&
      at.session === undefined &&
      (sessionLines()[0]?.runs ?? 0) > 0
      ? newest(room)
      : null;
  });
  const onClose = () => props.onClose();
  const viewingSession = createMemo(() => {
    const at = viewed();
    return at?.kind === "talk" ? at.session : undefined;
  });
  const allSessionRows = createMemo(() => query().trim() !== "");

  const entries = createMemo((): Entry[] => {
    const goTo = (to: View, label: string, hint = toFragment(to)): Entry => ({
      label,
      hint,
      to,
      act: () => {
        u.go(to);
      },
    });

    const page = (action: Action, to: View): Entry => ({
      ...goTo(to, say(lang(), LABELS[action])),
      keyAction: action,
    });

    const viewing = viewingSession();
    const room = here();
    const sessions = (
      room === null ? [] : sessionPlaces({ room, viewing }, sessionLines(), lang(), u.now())
    )
      .filter((place, at) => at < SESSIONS_AT_REST || allSessionRows())
      .map((place): Entry => {
        const origin = place.origin;
        const opening = origin === null ? null : branchFrom(branching(), room ?? MAYOR, origin);
        const note = opening === null ? null : noteOf(opening, room ?? MAYOR, lang(), null);
        return {
          label: place.label,
          hint: note?.said ?? place.hint,
          to: place.to,
          why: opening === null ? undefined : waitWhy(opening),
          keeps: origin === null ? undefined : true,
          act: () => {
            if (origin === null) u.go(place.to);
            else u.branches.open(room ?? MAYOR, { kind: "branch", origin }, onClose);
          },
        };
      });
    const carry: Entry[] =
      carryFrom() === null
        ? []
        : [
            {
              label: say(lang(), "palette_carry"),
              hint: say(lang(), "palette_carry_hint"),
              why: live() === null ? undefined : "palette_carry_working",
              keeps: true,
              act: () => {
                setCarrying(true);
                setQuery("");
                setCursor(0);
              },
            },
          ];
    const out: Entry[] = [
      ...sessions,
      ...carry,
      page("go.talk", { kind: "talk", address: MAYOR }),
      page("go.city", { kind: "city" }),
      page("go.setup", { kind: "setup" }),
      page("go.mcp", { kind: "mcp" }),
      page("go.registry", { kind: "registry" }),
      page("go.record", { kind: "record", lens: "ledger" }),
      goTo({ kind: "record", lens: "archive" }, say(lang(), "rec_archive")),
      goTo({ kind: "record", lens: "bin" }, say(lang(), "rec_bin")),
      page("go.cost", { kind: "cost" }),
      goTo({ kind: "welcome" }, say(lang(), "setup_rerun")),
      {
        label: say(lang(), LABELS.finder),
        hint: "",
        keyAction: "finder",
        keeps: true,
        act: () => {
          onClose();
          queueMicrotask(openFinder);
        },
      },
    ];

    if (room !== null && u.hearing() && canRecord()) {
      out.push({ label: say(lang(), "palette_transcribe"), hint: room, act: askToSpeak });
    }
    out.push({
      label: halted() ? RELEASE_ALL : say(lang(), "city_stop"),
      hint: "halt",

      act: () => {
        u.send(halted() ? release(CITY) : halt(CITY));
      },
    });
    for (const tongue of LANGS) {
      if (tongue !== held().lang) {
        out.push({
          label: endonym(tongue),
          hint: say(lang(), "setup_language"),
          act: () => {
            u.prefs.setLang(tongue);
          },
        });
      }
    }
    const answer = city();
    if (answer !== undefined && "city" in answer) {
      for (const building of answer.city.buildings) {
        out.push(
          building.adoptable
            ? goTo(
                { kind: "raise", address: building.addr },
                building.addr,
                say(lang(), "city_adoptable"),
              )
            : goTo(
                { kind: "building", address: building.addr },
                building.addr,
                say(lang(), "palette_building"),
              ),
        );
      }
    }
    for (const room of belief().rooms.keys()) {
      if (room === MAYOR) continue;
      const address = Option.getOrNull(Schema.decodeOption(Address)(room));
      if (address !== null) {
        out.push(goTo({ kind: "talk", address }, room, say(lang(), "palette_room")));
      }
    }
    return out;
  });

  const models = createMemo(() => {
    const answer = endpoints();
    if (answer === undefined || !("endpoints" in answer)) return [];
    return answer.endpoints.endpoints.flatMap((endpoint) =>
      endpoint.models.map((row) => ({ endpoint: endpoint.name, model: row.id })),
    );
  });
  function newest(room: string): Reached | null {
    return reached(heldIn(belief(), room).at(-1));
  }

  function runSlash(chosen: Slash): void {
    const written: { line: string | null; acted: boolean } = { line: null, acted: false };
    const hands: SlashHands = {
      command: (command) => {
        const sent = u.send(command);
        if (sent) written.acted = true;
        return sent;
      },
      go: (view) => {
        written.acted = true;
        u.go(view);
      },
      here: here(),
      live: live(),
      newest,
      buildings: buildings(),
      models: models(),
      effort: effort(),
      setEffort: (level) => {
        written.acted = true;
        u.chooseEffort(level);
      },
      policy: policy(),
      setPolicy: u.choosePolicy,
      goal: say(lang(), "talk_goal"),
      write: (line) => {
        written.line = line;
        setQuery(line);
        setCursor(0);
      },
      ...sessionHands(u, here(), viewingSession()),
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
    if (written.line === "" || (written.line === null && written.acted)) {
      onClose();
    }
  }

  const grouped = createMemo((): Group[] => {
    const typed = offered(query().trim());
    return SECTIONS.map((section) => ({
      section,
      entries: typed
        .filter((each) => each.section === section)
        .map((each) => ({
          label: each.grammar === "" ? each.spelling : `${each.spelling} ${each.grammar}`,
          hint: say(lang(), each.about),
          why: whyFor(each.spelling, {
            here: here(),
            live: live() !== null,
            models: models().length,
          }),
          act: () => {
            runSlash(each);
          },
        })),
    })).filter((group) => group.entries.length > 0);
  });
  const commands = createMemo(() => grouped().flatMap((group) => group.entries));

  const carried = createMemo((): Entry[] => {
    const from = carryFrom();
    const room = here();
    if (!carrying() || room === null || from === null) return [];
    const known = [...belief().rooms.keys()].flatMap((room) =>
      Option.toArray(Schema.decodeOption(Address)(room)),
    );
    return carryPlaces({
      here: room,
      rooms: known,
      typed: Option.getOrNull(Schema.decodeOption(Address)(query().trim())),
      working: (room) => newestWorking(belief(), room) !== undefined,
    }).map((place) => {
      const opening = branchInto(branching(), place.to);
      return {
        label: place.to,
        hint: noteOf(opening, place.to, lang(), null)?.said ?? "",
        why: place.why ?? waitWhy(opening),
        keeps: true,
        act: () => {
          u.branches.open(
            place.to,
            { kind: "branch", origin: { run: from.run, at_seq: from.at } },
            onClose,
          );
        },
      };
    });
  });

  const shown = createMemo((): Entry[] => {
    const needle = query().trim().toLowerCase();
    if (needle.startsWith("/") && !carrying()) {
      return commands();
    }
    const listed = carrying() ? carried() : entries();
    const direct =
      carrying() || needle.startsWith("/") ? null : directEntry(query(), buildings() ?? [], lang());
    const all: Entry[] =
      direct === null
        ? listed
        : [
            {
              ...direct,
              act: () => {
                u.go(direct.to);
              },
            },
            ...listed.filter(
              (entry) => entry.to === undefined || toFragment(entry.to) !== direct.hint,
            ),
          ];

    const searched = (entry: Entry): string =>
      carrying() ? entry.label : `${entry.label} ${entry.hint}`;
    return needle === ""
      ? all
      : all.filter((entry) => searched(entry).toLowerCase().includes(needle));
  });

  function pick(entry: Entry | undefined): void {
    if (entry === undefined || entry.why !== undefined) return;
    entry.act();

    if (entry.keeps !== true && (carrying() || !query().trim().startsWith("/"))) {
      onClose();
    }
  }

  const selected = createMemo(() => Math.max(0, Math.min(cursor(), shown().length - 1)));

  function write(line: string): void {
    setQuery(line);
    setCursor(0);
  }

  return {
    lang,
    here,
    query,
    cursor: selected,
    carrying,
    buildings,
    shown,
    grouped,
    write,
    point: setCursor,
    pick,
  };
}
