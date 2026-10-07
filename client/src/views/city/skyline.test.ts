// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import type { RunBelief } from "../../core/belief";
import type { Doing } from "../../core/doing";
import { Address, RunId, Seq, TimeMs, Tokens, UsdMicros } from "../../wire";
import type { BuildingProgress, CityAnswer } from "../../wire";
import { lookOf } from "./skyline";
import type { SkylineSeat } from "./skyline";

const IDLE = { unplanned: { budget: { tokens: Tokens.make(0), usd: UsdMicros.make(0) }, steps: 0 } };

function building(addr: string): BuildingProgress {
  return { addr: Address.make(addr), blocked: [], problems: [], progress: IDLE, ready: 0 };
}

function cityOf(...addrs: readonly string[]): CityAnswer {
  return { active: 0, frozen: 0, halted: [], pursuits: [], runs: [], buildings: addrs.map(building) };
}

function run(index: number, addr: string, doing: Doing): RunBelief {
  return {
    run: RunId.make(`00000000-0000-4000-8000-${index.toString(16).padStart(12, "0")}`),
    addr: Address.make(addr),
    started: TimeMs.make(index),
    task: null,
    goal: null,
    lastSeq: Seq.make(1),
    doing,
    model: null,
    pr: null,
    ask: null,
    local: false,
    saying: "",
    thinking: "",
  };
}

function seatOf(city: CityAnswer, runs: Readonly<Record<string, readonly RunBelief[]>> = {}): SkylineSeat {
  return { city, picked: null, runsIn: (addr) => runs[addr] ?? [], lang: "en", power: 2 };
}

describe("the skyline", () => {
  // A tower is a link: a click or Enter follows it, and Space, which a
  // link does not answer, picks nothing.
  test("a click or Enter on a tower picks its building, and no other key does", () => {
    const picks: string[] = [];
    const look = lookOf(seatOf(cityOf("hall", "docs")), (addr) => picks.push(addr));
    const docs = look.towers.find((tower) => tower.key === "docs");
    docs?.wire.onclick();
    docs?.wire.onkeydown({ key: " " });
    docs?.wire.onkeydown({ key: "Enter" });
    docs?.wire.onkeydown({ key: "ArrowRight" });
    expect(picks).toEqual(["docs", "docs"]);
    expect(docs?.wire.role).toBe("link");
    expect(docs?.wire.tabindex).toBe(0);
  });

  // The avenue reads outward from City Hall: the others alternate to its
  // right and left in name order.
  test("City Hall stands in the middle, the others alternating outward by name", () => {
    const look = lookOf(seatOf(cityOf("docs", "hall", "atlas", "web", "billing")), () => undefined);
    expect(look.towers.map((tower) => tower.key)).toEqual(["web", "billing", "hall", "atlas", "docs"]);
    expect(look.towers.map((tower) => tower.hall)).toEqual([false, false, true, false, false]);
  });

  // The newest run takes the first window; a run waiting for a person
  // and a run calling a tool are told apart from one simply at work,
  // and an ended run's window is no longer lit.
  test("each window says what the run behind it is doing, newest first", () => {
    const runs = {
      docs: [
        run(1, "docs", { kind: "frozen", completion: "done" }),
        run(2, "docs", { kind: "thinking" }),
        run(3, "docs", { kind: "waiting" }),
        run(4, "docs", { kind: "calling", tool: "read", subject: null }),
      ],
    };
    const look = lookOf(seatOf(cityOf("docs"), runs), () => undefined);
    const docs = look.towers[0];
    expect(docs?.cells.map((cell) => cell.pane)).toEqual(["calling", "waiting", "lit", "ended", "dark", "dark", "dark", "dark", "dark", "dark", "dark", "dark"]);
    expect(docs?.lit).toBe(true);
    expect(docs?.marks.map((mark) => mark.kind === "figure" && mark.posture)).toEqual(["thinking", "waiting", "calling"]);
    expect(docs?.wire["aria-label"]).toBe("docs · 3 at work");
  });

  // A name longer than its slot would run into the next tower's: what is
  // drawn stops at the slot with an ellipsis, and the whole name stays
  // in the label a pointer and a screen reader read.
  test("a long name is cut under its tower and kept whole in its title", () => {
    const long = "checkout-and-payments-gateway";
    const look = lookOf(seatOf(cityOf(long)), () => undefined);
    const name = look.towers[0]?.name;
    expect(name?.whole).toBe(long);
    expect(name?.shown).toBe("checkout-and-payments…");
    expect(look.towers[0]?.line).toBeNull();
  });
});
