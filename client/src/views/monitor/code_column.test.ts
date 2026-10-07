// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The code column's wiring, read without its look: which actions a hunk
// offers in which run, what each press sends, and the text of the two
// steers. The revert steer once reached the agent with its fences
// spelled `{fence}`, because the fill left that slot out; the agent
// was then asked to match a block that was never in the file.

import { describe, expect, test } from "bun:test";

import { Seq } from "../../wire";
import { lookOf, steerOf } from "./code_column";
import type { Hunk, Touched } from "./trace";

const hunk: Hunk = {
  lines: [
    { sign: "kept", old: 4, new: 4, text: "fn main() {" },
    { sign: "removed", old: 5, new: null, text: "    old();" },
    { sign: "added", old: null, new: 5, text: "    new();" },
  ],
};

const file: Touched = { path: "src/main.rs", how: "modified", at: Seq.make(7), hunks: [hunk] };

const NONE = { editor: "none", folder: "" } as const;

function sent(): { readonly drafts: string[]; readonly steers: string[]; readonly hands: { draft: (text: string) => void; steer: (text: string) => void } } {
  const drafts: string[] = [];
  const steers: string[] = [];
  return {
    drafts,
    steers,
    hands: {
      draft: (text) => drafts.push(text),
      steer: (text) => steers.push(text),
    },
  };
}

describe("the actions a hunk offers", () => {
  test("a live run offers comment and revert, and opening only where an editor answers a link", () => {
    const { hands } = sent();
    const bare = lookOf({ files: [file], live: true, editor: NONE }, "en", hands);
    expect(bare.files[0]?.hunks[0]?.actions.map((action) => action.key)).toEqual(["comment", "revert"]);
    const opened = lookOf({ files: [file], live: true, editor: { editor: "vscode", folder: "C:/city" } }, "en", hands);
    const open = opened.files[0]?.hunks[0]?.actions.at(-1);
    expect(open?.kind).toBe("open");
    expect(open?.kind === "open" ? open.wire.href : "").toContain("src/main.rs:4");
  });

  test("a finished run offers no steer, and a hunk with nothing to offer draws no actions", () => {
    const { hands } = sent();
    const look = lookOf({ files: [file], live: false, editor: NONE }, "en", hands);
    expect(look.files[0]?.hunks[0]?.actions).toEqual([]);
  });

  test("comment drafts and revert steers, each about the hunk's path and line", () => {
    const { drafts, steers, hands } = sent();
    const actions = lookOf({ files: [file], live: true, editor: NONE }, "en", hands).files[0]?.hunks[0]?.actions ?? [];
    for (const action of actions) if (action.kind === "press") action.wire.onclick();
    expect(drafts).toEqual(["About src/main.rs, line 4: "]);
    expect(steers).toHaveLength(1);
    expect(steers[0]).toContain("src/main.rs");
  });

  test("an empty run says so in place of the column", () => {
    const { hands } = sent();
    expect(lookOf({ files: [], live: true, editor: NONE }, "en", hands).empty).toBe("No file changed yet.");
  });
});

describe("the revert steer", () => {
  test("every slot is filled, the fences included", () => {
    for (const lang of ["en", "zh"] as const) {
      const steer = steerOf("mon_revert_steer", lang, file.path, hunk);
      expect(steer).not.toMatch(/\{(path|line|now|was|fence)\}/);
      expect(steer).toContain("```\nfn main() {\n    new();\n```");
      expect(steer).toContain("```\nfn main() {\n    old();\n```");
    }
  });

  test("a hunk's own text is never read as a slot or a replacement pattern", () => {
    const tricky: Hunk = { lines: [{ sign: "added", old: null, new: 1, text: "{path} $& {fence}" }] };
    expect(steerOf("mon_revert_steer", "en", "a.rs", tricky)).toContain("{path} $& {fence}");
  });
});
