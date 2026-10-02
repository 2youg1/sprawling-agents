// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { Option } from "effect";

import { Address, B3Hash, RunId, Seq } from "../wire";
||||||| parent of 22205091 (card-S8.SS4: red: sessions as stretches, their tags and pin, /compact /tag /untag, against stubs)
import { Address, RunId } from "../wire";
import {
  DEFAULT_VIEW,
  MAYOR,
  current,
  fromFragment,
  go,
  roomIn,
  SETUP_GROUPS,
  toFragment,
  unresolved,
  type View,
} from "./route";

const lab = Address.make("lab");
const parser = Address.make("lab/parser");
const seven = RunId.make("07070707-0707-0707-0707-070707070707");
const version = B3Hash.make("b3".padEnd(64, "0"));
const call = { kind: "call", run: seven, at: Seq.make(42) } as const;
const file = { kind: "document", building: lab, path: "docs/a plan?.md", version } as const;
const worktree = { kind: "document", building: lab, path: "README.md", version: null } as const;

// Every view this client has, so the round trip is exhaustive by
// construction: the lint's exhaustiveness check on `toFragment` refuses a
// variant this list does not spell.
const EVERY_VIEW: readonly View[] = [
  { kind: "talk", address: MAYOR },
  { kind: "talk", address: parser },
  { kind: "talk", address: parser, session: Seq.make(12) },
  { kind: "talk", address: MAYOR, session: Seq.make(3) },
  { kind: "city" },
  { kind: "welcome" },
  { kind: "record", lens: "ledger" },
  { kind: "record", lens: "archive" },
  { kind: "record", lens: "bin" },
  { kind: "cost" },
  { kind: "setup" },
  ...SETUP_GROUPS.map((group): View => ({ kind: "setup", group })),
  { kind: "building", address: lab },
  { kind: "run", run: seven },
  { kind: "talk", address: parser, item: call },
  { kind: "talk", address: parser, item: file },
  { kind: "talk", address: MAYOR, item: worktree },
  { kind: "run", run: seven, lens: "changes" },
  { kind: "welcome", step: "skills" },
];

describe("route", () => {
  test("every view survives the address bar unchanged", () => {
    for (const view of EVERY_VIEW) {
      const written = toFragment(view);
      expect(fromFragment(written), written).toEqual(Option.some(view));
    }
  });

  test("a stretch is named after the room by a character no address can hold", () => {
    expect(toFragment({ kind: "talk", address: parser, session: Seq.make(12) })).toBe("#/talk/lab/parser:12");
    for (const broken of ["#/talk/lab/parser:", "#/talk/lab/parser:x", "#/talk/lab/parser:-1", "#/talk/:4"]) {
      expect(fromFragment(broken), broken).toEqual(Option.none());
    }
  });

  test("every fragment is absolute so a hand-written one matches", () => {
    for (const view of EVERY_VIEW) {
      expect(toFragment(view).startsWith("#/")).toBe(true);
    }
  });

  test("nothing in the address bar is the first page", () => {
    for (const empty of ["", "#", "#/"]) {
      expect(fromFragment(empty), empty).toEqual(Option.some(DEFAULT_VIEW));
    }
    expect(DEFAULT_VIEW).toEqual({ kind: "talk", address: MAYOR });
  });

  test("a conversation is named by its room and not by a number", () => {
    expect(toFragment({ kind: "talk", address: parser })).toBe(
      "#/talk/lab/parser",
    );
    expect(fromFragment("#/talk/lab/parser")).toEqual(
      Option.some({ kind: "talk", address: parser }),
    );
    expect(fromFragment("#/s/lab/parser")).toEqual(
      Option.some({ kind: "talk", address: parser }),
    );
    expect(toFragment({ kind: "talk", address: MAYOR })).toBe("#/");
  });

  test("a room named in any script opens from the address bar the browser wrote", () => {
    const named = Address.make("shop/收到。");
    const written = "#/talk/shop/%E6%94%B6%E5%88%B0%E3%80%82";
    expect(fromFragment(written)).toEqual(
      Option.some({ kind: "talk", address: named }),
    );
    expect(fromFragment(toFragment({ kind: "talk", address: named }))).toEqual(
      Option.some({ kind: "talk", address: named }),
    );
    expect(fromFragment("#/talk/shop/%E6%94")).toEqual(Option.none());
  });

  test("every fragment the old pages wrote still lands", () => {
    const kept: readonly (readonly [string, View])[] = [
      ["#/overview", { kind: "city" }],
      ["#/sessions", DEFAULT_VIEW],
      ["#/live", DEFAULT_VIEW],
      ["#/approvals", DEFAULT_VIEW],
      ["#/waiting", DEFAULT_VIEW],
      ["#/ledger", { kind: "record", lens: "ledger" }],
      ["#/archive", { kind: "record", lens: "archive" }],
      ["#/recycle-bin", { kind: "record", lens: "bin" }],
      ["#/dashboard", { kind: "cost" }],
      ["#/settings", { kind: "setup" }],
      ["#/b/lab", { kind: "building", address: lab }],
      ["#/live/07070707-0707-0707-0707-070707070707", { kind: "run", run: seven }],
    ];
    for (const [fragment, landing] of kept) {
      expect(fromFragment(fragment), fragment).toEqual(Option.some(landing));
    }
  });

  test("no view writes a fragment this build no longer uses", () => {
    const retired = [
      "#/overview",
      "#/sessions",
      "#/waiting",
      "#/approvals",
      "#/ledger",
      "#/archive",
      "#/recycle-bin",
      "#/dashboard",
      "#/settings",
      "#/b/lab",
      "#/s/lab/parser",
    ];
    for (const view of EVERY_VIEW) {
      expect(retired, toFragment(view)).not.toContain(toFragment(view));
    }
  });

  test("a fragment that names nothing answers nothing", () => {
    for (const wrong of [
      "#/nowhere",
      "#/s/",
      "#/b/",
      "#/building/",
      "#/run/not-a-run",
      "#/live/not-a-run",
      "#/city/extra",
      "#/record/nowhere",
      "#/welcome/extra",
      "#/setup/nowhere",
      "#/setup/you/extra",
    ]) {
      expect(fromFragment(wrong), wrong).toEqual(Option.none());
    }
  });

  test("the settings panel opens at the group its address names", () => {
    expect(fromFragment("#/setup/you")).toEqual(Option.some({ kind: "setup", group: "you" }));
    expect(toFragment({ kind: "setup", group: "rules" })).toBe("#/setup/rules");
    expect(toFragment({ kind: "setup" })).toBe("#/setup");
  });

  test("a pairing invitation opens the remote group, readable or not", () => {
    const remote: View = { kind: "setup", group: "remote" };
    expect(fromFragment("#pair=abcdefghijklmnopqrstuvwxyz&city=aaaa")).toEqual(Option.some(remote));
    expect(fromFragment("#pair=")).toEqual(Option.some(remote));
    expect(unresolved("#pair=abc&city=def")).toEqual(Option.none());
  });

  test("an unresolved fragment is reported by name, an empty one is not", () => {
    expect(unresolved("")).toEqual(Option.none());
    expect(unresolved("#/")).toEqual(Option.none());
    expect(unresolved("#/cost")).toEqual(Option.none());
    expect(unresolved("#/nowhere")).toEqual(Option.some("#/nowhere"));
    expect(unresolved("#nowhere/deep")).toEqual(Option.some("#/nowhere/deep"));
  });

  test("the address bar is read and written through one door", () => {
    const bar = { hash: "#/settings" };
    expect(current(bar)).toEqual(Option.some({ kind: "setup" }));
    go(bar, { kind: "record", lens: "bin" });
    expect(bar.hash).toBe("#/record/bin");
    expect(current(bar)).toEqual(Option.some({ kind: "record", lens: "bin" }));
  });
});

describe("deep links", () => {
  test("a call and a document version are spelled after the conversation they open beside", () => {
    expect([
      toFragment({ kind: "talk", address: parser, item: call }),
      toFragment({ kind: "talk", address: MAYOR, item: worktree }),
      toFragment({ kind: "talk", address: parser, item: file }),
    ]).toEqual([
      `#/talk/lab/parser?call=${seven}&at=42`,
      "#/talk/hall/mayor?building=lab&path=README.md",
      `#/talk/lab/parser?building=lab&path=docs%2Fa+plan%3F.md&version=${version}`,
    ]);
  });

  test("a run lens and a guide step are a path segment", () => {
    expect([toFragment({ kind: "run", run: seven, lens: "turns" }), toFragment({ kind: "welcome", step: "mcp" })]).toEqual([
      `#/run/${seven}/turns`,
      "#/welcome/mcp",
    ]);
  });

  test("a locator with a key missing, a key too many or a value the wire refuses names nothing", () => {
    expect(
      [
        `#/talk/lab/parser?call=${seven}`,
        `#/talk/lab/parser?call=${seven}&at=42&path=x`,
        `#/talk/lab/parser?call=${seven}&at=4.5`,
        "#/talk/lab/parser?call=nope&at=1",
        "#/talk/lab/parser?building=lab&path=",
        "#/talk/lab/parser?building=lab&path=a&version=short",
        "#/talk/lab/parser?",
        `#/city?call=${seven}&at=1`,
        `#/run/${seven}/nowhere`,
        "#/welcome/nowhere",
      ].map((raw) => Option.isNone(fromFragment(raw))),
    ).toEqual([true, true, true, true, true, true, true, true, true, true]);
  });

  test("a room whose name holds a question mark keeps it inside the address", () => {
    const odd = Address.make("lab/what?");
    expect(toFragment({ kind: "talk", address: odd })).toBe("#/talk/lab/what%3F");
    expect(fromFragment("#/talk/lab/what%3F")).toEqual(Option.some({ kind: "talk", address: odd }));
  });
});

describe("naming a room in a building", () => {
  test("a name the city takes is the room's address, and one it refuses is none", () => {
    expect([roomIn(lab, " first try "), roomIn(lab, "a/b"), roomIn(lab, "   "), roomIn(lab, "dots.")]).toEqual([
      Option.some(Address.make("lab/first try")),
      Option.none(),
      Option.none(),
      Option.none(),
    ]);
  });
});
