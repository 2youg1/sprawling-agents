// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import type { AxError } from "../../wire";
import { NONE, delivered, sent } from "./delivery";
import type { Heard } from "./delivery";

const refusal: AxError = {
  action: "dispatch",
  code: "E_MODEL_UNCHOSEN",
  nearby: [],
  recovery: "",
  retry: "no",
  subject: "hall/mayor",
};

const live = (newest: number, refused: AxError | null = null): Heard => ({ live: true, newest, refusal: refused });
const down = (newest: number): Heard => ({ live: false, newest, refusal: null });

describe("where sent words stand", () => {
  test("sent on a live link they wait for the city, and a newer record accepts them", () => {
    const out = sent("fix it", live(10));
    expect(out).toEqual({ kind: "pending", words: "fix it", refusal: null, mark: 10 });
    expect(delivered(out, live(10))).toEqual(out);
    expect(delivered(out, live(11))).toEqual({ kind: "accepted", words: "fix it" });
  });

  test("a link that drops while they wait makes them unknown, and nothing sends them again", () => {
    const lostOnTheWay = delivered(sent("fix it", live(10)), down(10));
    expect(lostOnTheWay).toEqual({ kind: "unknown", words: "fix it", refusal: null, mark: 10 });
    expect(delivered(lostOnTheWay, live(10))).toEqual(lostOnTheWay);
    expect(delivered(lostOnTheWay, live(12))).toEqual({ kind: "accepted", words: "fix it" });
  });

  test("sent with the link down they are held, and go out when it is back", () => {
    const held = sent("fix it", down(10));
    expect(held).toEqual({ kind: "held", words: "fix it", refusal: null });
    expect(delivered(held, down(10))).toEqual(held);
    expect(delivered(held, live(10))).toEqual({ kind: "pending", words: "fix it", refusal: null, mark: 10 });
  });

  test("a refusal newer than the send ends the echo; an older one does not", () => {
    const out = sent("fix it", live(10, refusal));
    expect(delivered(out, live(10, refusal))).toEqual(out);
    expect(delivered(out, live(10, { ...refusal, code: "E_PROVIDER" }))).toEqual(NONE);
  });
});
