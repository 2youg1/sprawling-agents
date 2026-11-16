// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import type { AxError, DoctorAnswer, DoctorItem } from "../../wire";
import { answered, over, refused, running, started } from "./installing";

const item = (name: string, here: boolean): DoctorItem => ({
  name,
  install: { command: { spelled: `cargo install ${name} --locked` } },
  need: "required",
  tier: "develop",
  state: here
    ? { present: { at: name, version: "silent" } }
    : { absent: { absence: "not_on_search_path" } },
});

const machine = (items: readonly DoctorItem[]): DoctorAnswer => ({
  items: [...items],
  tiers: [],
  sandbox: { arm: "copied_tree", named: "copied_tree", coverage: [] },
  custody: { store: "session_memory", keeps: "this_process", refusal: null },
  core: "raised",
  scanning: "does_not_apply",
});

const refusal = (subject: string): AxError => ({
  action: "install a tool",
  code: "E_TOOL_UNAVAILABLE",
  nearby: [],
  recovery: "read what it reported in the log",
  retry: "no",
  subject,
});

describe("the install-all walk", () => {
  test("moves on at a fresh answer that has the item, and at a refusal that names it", () => {
    let walk = started(["just", "cargo-deny", "bun"]);
    expect(running(walk)).toBe("just");

    walk = answered(walk, machine([item("just", false)]));
    expect(running(walk)).toBe("just");

    walk = answered(walk, machine([item("just", true)]));
    expect(running(walk)).toBe("cargo-deny");

    walk = refused(walk, refusal("bun: somebody else's failure"));
    expect(running(walk)).toBe("cargo-deny");

    walk = refused(walk, refusal("cargo-deny: cargo install cargo-deny --locked ended in failure"));
    walk = answered(walk, machine([item("bun", true)]));
    expect(walk.map((step) => [step.name, step.state])).toEqual([
      ["just", "done"],
      ["cargo-deny", "failed"],
      ["bun", "done"],
    ]);
    expect(over(walk)).toBe(true);
  });
});
