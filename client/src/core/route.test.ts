// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { Option } from "effect";

import { Address } from "./address";
import {
  DEFAULT_VIEW,
  current,
  fromFragment,
  go,
  toFragment,
  unresolved,
  type View,
} from "./route";
import { RunId } from "./run_id";

const lab = Option.getOrThrow(Address.option("lab"));
const parser = Option.getOrThrow(Address.option("lab/parser"));
const seven = Option.getOrThrow(
  RunId.option("07070707-0707-0707-0707-070707070707"),
);

// Every view this client has, so the round trip is exhaustive by
// construction: the lint's exhaustiveness check on `toFragment` refuses a
// variant this list does not spell.
const EVERY_VIEW: readonly View[] = [
  { kind: "sessions" },
  { kind: "session", address: parser },
  { kind: "waiting" },
  { kind: "record", lens: "ledger" },
  { kind: "record", lens: "archive" },
  { kind: "record", lens: "bin" },
  { kind: "cost" },
  { kind: "setup" },
  { kind: "building", address: lab },
  { kind: "run", run: seven },
];

describe("route", () => {
  test("every view survives the address bar unchanged", () => {
    for (const view of EVERY_VIEW) {
      const written = toFragment(view);
      expect(fromFragment(written), written).toEqual(Option.some(view));
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
    expect(DEFAULT_VIEW).toEqual({ kind: "sessions" });
  });

  test("a session is named by its room and not by a number", () => {
    expect(toFragment({ kind: "session", address: parser })).toBe(
      "#/s/lab/parser",
    );
    expect(fromFragment("#/s/lab/parser")).toEqual(
      Option.some({ kind: "session", address: parser }),
    );
  });

  test("every fragment the old pages wrote still lands", () => {
    const kept: readonly (readonly [string, View])[] = [
      ["#/overview", { kind: "sessions" }],
      ["#/city", { kind: "sessions" }],
      ["#/live", { kind: "sessions" }],
      ["#/approvals", { kind: "waiting" }],
      ["#/ledger", { kind: "record", lens: "ledger" }],
      ["#/archive", { kind: "record", lens: "archive" }],
      ["#/recycle-bin", { kind: "record", lens: "bin" }],
      ["#/dashboard", { kind: "cost" }],
      ["#/settings", { kind: "setup" }],
      ["#/building/lab", { kind: "building", address: lab }],
      ["#/live/07070707-0707-0707-0707-070707070707", { kind: "run", run: seven }],
    ];
    for (const [fragment, landing] of kept) {
      expect(fromFragment(fragment), fragment).toEqual(Option.some(landing));
    }
  });

  test("no view writes a fragment this build no longer uses", () => {
    const retired = [
      "#/overview",
      "#/city",
      "#/approvals",
      "#/ledger",
      "#/archive",
      "#/recycle-bin",
      "#/dashboard",
      "#/settings",
      "#/building/lab",
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
      "#/live/not-a-run",
      "#/city/extra",
      "#/record/nowhere",
      "#/waiting/extra",
    ]) {
      expect(fromFragment(wrong), wrong).toEqual(Option.none());
    }
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
