// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { get } from "svelte/store";

import { createBelief } from "../../core/belief";
import type { AxError, EventRecord } from "../../wire";
import { B3Hash, RunId, Seq, TimeMs } from "../../wire";
import { IDLE, NOT_CONVERSING, claims, hand, settle } from "./handing";

const WORDS = "add a test for price";

const NO_MODEL: AxError = {
  action: "choose the main model",
  code: "E_CONFIG_INVALID",
  nearby: [],
  recovery: "attach a provider on the settings page and pick a model for this tag",
  retry: "no",
  subject: "no model is chosen for this tag",
};

function started(at: number): EventRecord {
  return {
    run: RunId.make("11111111-1111-4111-8111-111111111111"),
    seq: Seq.make(at),
    kind: "run_started",
    t: TimeMs.make(at),
    who: "hall/mayor",
    prev: B3Hash.make("0".repeat(64)),
    v: 1,
    data: { task: WORDS },
  };
}

describe("the words a box handed to the city", () => {
  test("a refused line goes back into the empty box", () => {
    const store = createBelief(() => 0);
    const handing = hand(WORDS, get(store.belief));
    store.refused(NO_MODEL);
    expect(settle(handing, get(store.belief), "")).toEqual({ handing: IDLE, box: WORDS });
  });

  test("a refused line goes ahead of what was typed since", () => {
    const store = createBelief(() => 0);
    const handing = hand(WORDS, get(store.belief));
    store.refused(NO_MODEL);
    expect(settle(handing, get(store.belief), "and one for tax")).toEqual({
      handing: IDLE,
      box: `${WORDS}\nand one for tax`,
    });
  });

  test("a line the city recorded after is not given back", () => {
    const store = createBelief(() => 0);
    const handing = hand(WORDS, get(store.belief));
    store.apply(started(4));
    const accepted = settle(handing, get(store.belief), "");
    expect(accepted).toEqual({ handing: IDLE, box: null });
    store.refused(NO_MODEL);
    expect(settle(accepted.handing, get(store.belief), "")).toEqual({ handing: IDLE, box: null });
  });

  test("a refusal older than the send leaves the words handed", () => {
    const store = createBelief(() => 0);
    store.refused(NO_MODEL);
    const handing = hand(WORDS, get(store.belief));
    expect(settle(handing, get(store.belief), "")).toEqual({ handing, box: null });
  });
});

describe("which refusal the conversation draws itself", () => {
  test("a conversation still waiting claims the refusal that settles its words", () => {
    const store = createBelief(() => 0);
    const handing = hand(WORDS, get(store.belief));
    store.refused(NO_MODEL);
    const now = get(store.belief);
    expect(claims({ kind: "waiting", handing }, now.refusal ?? NO_MODEL, now)).toBe(true);
  });

  test("a conversation that drew the refusal names it", () => {
    const store = createBelief(() => 0);
    store.refused(NO_MODEL);
    const now = get(store.belief);
    const drawn = now.refusal ?? NO_MODEL;
    expect(claims({ kind: "answered", refusal: drawn }, drawn, now)).toBe(true);
  });

  test("a refusal nobody in the conversation waited on stays the corner's", () => {
    const store = createBelief(() => 0);
    store.refused(NO_MODEL);
    const now = get(store.belief);
    expect(claims(NOT_CONVERSING, now.refusal ?? NO_MODEL, now)).toBe(false);
  });
});
