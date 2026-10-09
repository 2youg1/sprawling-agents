// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The settings row's wiring, read off the value its look is given
// (client/Spec.lean §4-60): which menu a fixture opens, when a binding
// the popover hands over again is the one held, and how the permissions
// panel's switches and selects change one field of the policy each. The
// model picker's half is `picker.test.ts`. No look is imported.

import { expect, test } from "bun:test";

import { FIRST_POLICY } from "../../core/commands";
import type { PopoverBinding } from "../parts/popover";
import { permissionsOf } from "./policy";
import type { PermissionsHands } from "./policy";
import { menuOf, sameBinding } from "./settings_row";

const nothing = (): (() => void) => () => undefined;

test("the fixture's starting menu is the only one a row opens without a hand", () => {
  expect([menuOf("open"), menuOf("model"), menuOf("provider"), menuOf("level"), menuOf("closed")]).toEqual(["policy", "model", "model", "model", null]);
});

test("a binding handed over again with the same keys and lists is the one already held", () => {
  const keys = (): boolean => true;
  const held: PopoverBinding = { keys, pointColumn: nothing, controls: ["a", "b"] };
  expect([
    sameBinding(null, held),
    sameBinding(held, { ...held, controls: ["a", "b"] }),
    sameBinding(held, { ...held, controls: ["a", "c"] }),
    sameBinding(held, { ...held, keys: () => false }),
  ]).toEqual([false, true, false, false]);
});

function permissions(open: boolean): { readonly look: ReturnType<typeof permissionsOf>; readonly calls: string[]; readonly chosen: unknown[] } {
  const calls: string[] = [];
  const chosen: unknown[] = [];
  const hands: PermissionsHands = {
    choose: (policy) => chosen.push(policy),
    toggle: () => calls.push("toggle"),
    close: (focus) => calls.push(`close ${focus}`),
    inside: () => false,
    hold: { frame: nothing, trigger: nothing, first: nothing },
  };
  return { look: permissionsOf("en", { ...FIRST_POLICY, admit: "tested" }, open, hands), calls, chosen };
}

test("each switch and select of the permissions panel changes its own field alone", () => {
  const { look, chosen } = permissions(true);
  const panel = look.panel;
  expect(panel?.switches.map((each) => [each.key, each.wire.role, each.wire.checked])).toEqual([
    ["mode", "switch", FIRST_POLICY.mode === "work"],
    ["write", "switch", FIRST_POLICY.write === "full"],
  ]);
  expect(panel?.selects.map((each) => [each.key, each.wire.value])).toEqual([
    ["admit", "tested"],
    ["landing", FIRST_POLICY.landing],
  ]);
  panel?.switches[0]?.wire.onchange({ currentTarget: { checked: true } });
  panel?.switches[1]?.wire.onchange({ currentTarget: { checked: true } });
  panel?.selects[1]?.wire.onchange({ currentTarget: { value: "experiment" } });
  expect(chosen).toEqual([
    { ...FIRST_POLICY, admit: "tested", mode: "work" },
    { ...FIRST_POLICY, admit: "tested", write: "full" },
    { ...FIRST_POLICY, admit: "tested", landing: "experiment" },
  ]);
});

test("the permissions panel closes onto its entry on Escape and lets the focus go when it leaves", () => {
  const { look, calls } = permissions(true);
  const marks: string[] = [];
  look.panel?.wire.onkeydown({ key: "Escape", preventDefault: () => marks.push("prevented"), stopPropagation: () => marks.push("stopped") });
  look.panel?.wire.onkeydown({ key: "Enter", preventDefault: () => marks.push("prevented"), stopPropagation: () => marks.push("stopped") });
  look.frame.onfocusout({ relatedTarget: null });
  expect({ calls, marks }).toEqual({ calls: ["close opener", "close leave"], marks: ["prevented", "stopped"] });
  expect([look.trigger["aria-expanded"], look.trigger["aria-haspopup"]]).toEqual([true, "dialog"]);
  expect(permissions(false).look.panel).toBeUndefined();
});
