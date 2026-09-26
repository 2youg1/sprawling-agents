// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { get } from "svelte/store";

import table from "../lang.json";
import { Address, RunId, Seq } from "../wire";
import type { Command, Origin } from "../wire";
import type { View } from "./route";
import { SLASH, find, offered, parse } from "./slash";
import type { Slash, SlashCall, SlashHands } from "./slash";
import { completed } from "./completion";
import { forkAsked } from "./forking";

describe("slash", () => {
  test("every verb is spelled once, with a slash", () => {
    const seen = new Set<string>();
    for (const known of SLASH) {
      expect(known.spelling, "a verb begins with a slash").toMatch(/^\/[a-z]+$/);
      expect(seen.has(known.spelling), `${known.spelling} is spelled twice`).toBe(false);
      expect(["actions", "navigation", "sessions"], `${known.spelling} names its palette section`).toContain(
        known.section,
      );
      seen.add(known.spelling);
    }
  });

  // The defect §4.3 names: the pages label their buttons with command
  // spellings, and a person who typed one of those labels into the box
  // got nothing. A label and a verb are the same table or they are two,
  // and two is how they drifted apart.
  test("every command a button is labelled with is a verb somebody can type", () => {
    for (const [key, phrase] of Object.entries(table)) {
      if (!phrase.en.startsWith("/")) {
        continue;
      }
      const call = parse(phrase.en);
      expect(call, `${key} is not a command line`).not.toBeNull();
      expect(find(call?.verb ?? ""), `${key} spells a verb no table holds`).toBeDefined();
    }
  });

  test("a line is cut into the verb, its words, and the sentence after it", () => {
    expect(parse("hello")).toBeNull();
    expect(parse("/stop")).toEqual({ verb: "/stop", words: [], rest: "" });
    expect(parse("/stop --all")).toEqual({ verb: "/stop", words: ["--all"], rest: "--all" });
    expect(parse("/steer  read the city ")).toEqual({
      verb: "/steer",
      words: ["read", "the", "city"],
      rest: "read the city",
    });
  });

  test("the menu narrows as the verb is typed, then shows the one verb's grammar", () => {
    expect(offered("hello")).toHaveLength(0);
    expect(offered("/").length).toBe(SLASH.length);
    expect(offered("/st").map((each) => each.spelling)).toEqual(["/steer", "/stop"]);
    expect(offered("/stop ").map((each) => each.spelling)).toEqual(["/stop"]);
  });

  test("tab completes to the one match, or to the prefix every match shares", () => {
    expect(completed("/dis")).toBe("/dispatch ");
    // `/steer` and `/stop` share only the letter already typed, so the
    // line is returned untouched rather than silently shortened.
    expect(completed("/st")).toBe("/st");
    expect(completed("/m")).toBe("/m");
    expect(completed("/mc")).toBe("/mcp ");
    expect(completed("/zzz")).toBe("/zzz");
    expect(completed("read the city")).toBe("read the city");
  });
});

// `/new` and `/fork` are one verb with and without an origin: both send
// the same `OpenSession`, and the difference is what the new session
// inherits. These cases hold that seam - the picker request, the carry
// flag, and the one place `from` is built.
describe("new and fork", () => {
  const MOTHER = RunId.make("00000000-0000-0000-0000-000000000001");

  function called(line: string): SlashCall {
    const held = parse(line);
    expect(held, `${line} is a command line`).not.toBeNull();
    return held ?? { verb: "", words: [], rest: "" };
  }

  function verb(spelling: string): Slash {
    const known = find(spelling);
    expect(known, `${spelling} is a verb the table holds`).toBeDefined();
    return (
      known ?? {
        spelling,
        grammar: "",
        about: "slash_help",
        section: "actions",
        run: () => undefined,
      }
    );
  }

  // Every capability a verb may reach for, recording what it sent and
  // answering `newest` for the one room named below.
  function hands(
    here: Address | null,
    newest: (room: string) => { run: RunId; at: Seq } | null,
    live: { run: RunId; at: Seq } | null = null,
  ): { filled: SlashHands; sent: Command[]; written: string[]; went: View[] } {
    const sent: Command[] = [];
    const written: string[] = [];
    const went: View[] = [];
    return {
      sent,
      written,
      went,
      filled: {
        command: (command) => {
          sent.push(command);
          return true;
        },
        go: (view) => went.push(view),
        here,
        live,
        newest,
        models: [],
        effort: null,
        setEffort: () => undefined,
        mode: "plan_goal",
        goal: "a goal",
        write: (line) => written.push(line),
      },
    };
  }

  // What the sent frames asked the city to open, or `null` when none
  // did. One reader, because every case below asks the same question.
  function opened(
    sent: readonly Command[],
  ): { addr: Address; carry: string; from: Origin | null } | null {
    for (const frame of sent) {
      if ("open_session" in frame) {
        return {
          addr: frame.open_session.addr,
          carry: frame.open_session.carry,
          from: frame.open_session.from ?? null,
        };
      }
    }
    return null;
  }

  test("/dispatch runs in the mode the person chose, not a fixed one", () => {
    const held = hands(Address.make("lab/room1"), () => null);
    verb("/dispatch").run({ ...held.filled, mode: "up" }, called("/dispatch add the parser"));
    const frame = held.sent[0];
    expect(frame !== undefined && "dispatch" in frame ? frame.dispatch.mode : null).toBe("up");
  });

  test("/new opens a session here and carries nothing by default", () => {
    const room = Address.make("hall/mayor");
    const held = hands(room, () => null);
    verb("/new").run(held.filled, called("/new"));
    expect(opened(held.sent)).toEqual({ addr: room, carry: "nothing", from: null });
    expect(held.written).toEqual([""]);
  });

  test("/new --carry brings the handoff along, and says so", () => {
    const room = Address.make("lab/room1");
    const held = hands(room, () => null);
    verb("/new").run(held.filled, called("/new --carry"));
    expect(opened(held.sent)).toEqual({ addr: room, carry: "handoff", from: null });
  });

  test("/new outside a room sends nothing", () => {
    const held = hands(null, () => null);
    verb("/new").run(held.filled, called("/new"));
    expect(held.sent).toHaveLength(0);
  });

  test("/fork with no argument asks for the picker and clears the line", () => {
    const before = get(forkAsked);
    const held = hands(Address.make("hall/mayor"), () => null);
    verb("/fork").run(held.filled, called("/fork"));
    expect(get(forkAsked), "the talk screen watches this count").toBe(before + 1);
    expect(held.sent, "no frame goes out: the picker names the point").toHaveLength(0);
    expect(held.written).toEqual([""]);
  });

  test("/fork with an address branches that room's newest run at its tail", () => {
    const target = Address.make("lab/room1");
    const held = hands(Address.make("hall/mayor"), (room) =>
      room === target ? { run: MOTHER, at: Seq.make(7) } : null,
    );
    verb("/fork").run(held.filled, called("/fork lab/room1"));
    expect(opened(held.sent)).toEqual({
      addr: target,
      carry: "nothing",
      from: { run: MOTHER, at_seq: Seq.make(7) },
    });
  });

  test("/fork naming a room with no run does nothing", () => {
    const held = hands(Address.make("hall/mayor"), () => null);
    verb("/fork").run(held.filled, called("/fork lab/room1"));
    expect(held.sent).toHaveLength(0);
    expect(held.written, "the line stays for editing").toHaveLength(0);
  });
  // Halt and Cancel are two verbs in the glossary, so they are two
  // spellings here: `/stop` ends the run in front of the person and
  // nothing wider, and `/halt` is the brake `/release` lifts.
  test("/stop cancels the run in front of the person and never halts", () => {
    const held = hands(Address.make("hall/mayor"), () => null, { run: MOTHER, at: Seq.make(3) });
    verb("/stop").run(held.filled, called("/stop --all"));
    expect(held.sent.map((frame) => ("cancel" in frame ? frame.cancel.run : null))).toEqual([MOTHER]);
  });

  test("/halt pairs with /release over a building and the city", () => {
    const held = hands(Address.make("hall/mayor"), () => null);
    verb("/halt").run(held.filled, called("/halt lab"));
    verb("/halt").run(held.filled, called("/halt --all"));
    verb("/release").run(held.filled, called("/release lab"));
    expect(held.sent.map((frame) => Object.keys(frame).at(0))).toEqual(["halt", "halt", "release"]);
    expect(find("/halt")?.grammar).toBe(find("/release")?.grammar);
  });

  test("/halt with a word that is not an address sends nothing and keeps the line", () => {
    const held = hands(Address.make("hall/mayor"), () => null);
    verb("/halt").run(held.filled, called("/halt lab:"));
    verb("/release").run(held.filled, called("/release --al"));
    expect({ sent: held.sent, written: held.written }).toEqual({ sent: [], written: [] });
  });

  test("/clear drops the conversation the way /new does", () => {
    const room = Address.make("hall/mayor");
    const held = hands(room, () => null);
    verb("/clear").run(held.filled, called("/clear"));
    expect(opened(held.sent)).toEqual({ addr: room, carry: "nothing", from: null });
  });

  test("/steer takes only the sentence", () => {
    expect(find("/steer")?.grammar).toBe("<text>");
  });

  test("/diff opens the run page that holds the changes lens", () => {
    const room = Address.make("lab/room1");
    const held = hands(room, (asked) => (asked === room ? { run: MOTHER, at: Seq.make(7) } : null));
    verb("/diff").run(held.filled, called("/diff"));
    expect(held.went).toEqual([{ kind: "run", run: MOTHER }]);
  });
});
