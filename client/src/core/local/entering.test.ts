// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { browserLabel, mayHaveDoor, openCodeIn, withoutOpenCode } from "./entering";

// The open code is a secret for as long as it is unredeemed: the page
// must read it only from the fragment `/web` wrote, and leave an address
// that no longer carries it.
describe("the open code", () => {
  test("is read from the fragment /web writes, and from nothing else", () => {
    expect(["#open=c0de", "#open=", "#/talk/hall%2Fmayor", "", "#token=c0de"].map(openCodeIn)).toEqual([
      "c0de",
      null,
      null,
      null,
      null,
    ]);
  });

  test("leaves an address with the same path and query and no fragment", () => {
    expect(withoutOpenCode({ pathname: "/", search: "?lang=zh" })).toBe("/?lang=zh");
  });

  test("is looked for only where this machine's door can be", () => {
    expect(["http:", "https:", "file:"].map(mayHaveDoor)).toEqual([true, false, false]);
  });
});

describe("the name a browser pairs under", () => {
  test("names the browser before the engine it shares a token with", () => {
    const edge = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36 Edg/130.0";
    const safari = "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_5) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.5 Safari/605.1.15";
    expect([browserLabel(edge), browserLabel(safari), browserLabel("")]).toEqual(["Edge · Windows", "Safari · macOS", "Browser"]);
  });
});
