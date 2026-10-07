// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The settings row's wiring, read off the value its look is given
// (client/Spec.lean §4-60): where a pick in the model panel moves the
// focus, which keys the search box keeps and which it hands to the
// popover's key table, and how the permissions panel's switches and
// selects change one field of the policy each. No look is imported.

import { expect, test } from "bun:test";

import { FIRST_POLICY } from "../../core/commands";
import type { PopoverBinding } from "../parts/popover";
import type { Pill } from "./composer";
import { FILTER_AFTER, modelValue } from "./composer";
import { permissionsOf } from "./policy";
import type { PermissionsHands } from "./policy";
import { COLUMN, menuOf, modelOf, searchKeeps } from "./settings_row";
import type { ModelHands, ModelHeld } from "./settings_row";

const nothing = (): (() => void) => () => undefined;

function pill(values: readonly string[], value: string | null, picked: string[]): Pill {
  return {
    label: "Model",
    placeholder: "No model",
    choices: values.map((each) => ({ value: each, label: each })),
    value,
    pick: (chosen) => picked.push(chosen),
  };
}

// A model value as the composer spells it: its provider, then its name.
const valued = (endpoint: string, name: string): string => modelValue({ endpoint, model: name }) ?? "";

// A provider "big" serving more models than the filter threshold, and
// a provider "small" serving one.
const BIG = Array.from({ length: FILTER_AFTER + 1 }, (_unused, index) => `model-${String(index)}`);

function model(held: Partial<ModelHeld>): { readonly look: ReturnType<typeof modelOf>; readonly calls: string[]; readonly picked: string[] } {
  const calls: string[] = [];
  const picked: string[] = [];
  const hands: ModelHands = {
    toggle: () => calls.push("toggle"),
    close: () => calls.push("close"),
    point: (provider) => calls.push(`point ${provider}`),
    query: (text) => calls.push(`query ${text}`),
    bound: () => calls.push("bound"),
    cursor: (id) => calls.push(`cursor ${String(id)}`),
    focus: { trigger: () => calls.push("focus trigger"), search: () => calls.push("focus search") },
    hold: { trigger: nothing, search: nothing },
  };
  const look = modelOf(
    { lang: "en", models: pill([...BIG.map((name) => valued("big", name)), valued("small", "model-a")], valued("big", "model-0"), picked), effort: pill(["low", "high"], "high", picked) },
    { open: true, pointed: null, query: "", binding: null, active: null, ...held },
    hands,
  );
  return { look, calls, picked };
}

test("the fixture's starting menu is the only one a row opens without a hand", () => {
  expect([menuOf("open"), menuOf("model"), menuOf("closed")]).toEqual(["policy", "model", null]);
});

test("no provider offers a model, so the model entry is hidden", () => {
  const look = modelOf(
    { lang: "en", models: pill([], null, []), effort: pill(["low"], "low", []) },
    { open: false, pointed: null, query: "", binding: null, active: null },
    { toggle: nothing, close: nothing, point: nothing, query: nothing, bound: nothing, cursor: nothing, focus: { trigger: nothing, search: nothing }, hold: { trigger: nothing, search: nothing } },
  );
  expect(look).toBeUndefined();
});

test("a pick in the model panel keeps the focus in the panel, on the entry when the search box goes", () => {
  const { look, calls, picked } = model({});
  const menu = look?.menu;
  const column = (id: string) => menu?.columns.find((each) => each.id === id);
  const provider = column(COLUMN.provider);
  const models = column(COLUMN.model);
  const effort = column(COLUMN.effort);
  expect([provider?.id, models?.id, effort?.id]).toEqual([COLUMN.provider, COLUMN.model, COLUMN.effort]);
  if (menu === undefined || provider === undefined || models === undefined || effort === undefined) return;
  menu.onApply(provider, { id: "small", label: "small" });
  menu.onApply(provider, { id: "big", label: "big" });
  menu.onApply(models, { id: valued("big", "model-3"), label: "model-3" });
  menu.onApply(effort, { id: "low", label: "low" });
  expect({ calls, picked }).toEqual({
    calls: ["focus trigger", "point small", "focus search", "point big", "focus search", "focus search"],
    picked: [valued("big", "model-3"), "low"],
  });
});

test("the search box keeps Home, End and composing keys, and hands the other menu keys to the popover", () => {
  const binding: PopoverBinding = { keys: () => true, controls: ["list-a", "list-b"], pointColumn: nothing };
  const search = model({ binding, active: "row-4" }).look?.search;
  expect([search?.["aria-controls"], search?.["aria-activedescendant"]]).toEqual(["list-a list-b", "row-4"]);
  expect([searchKeeps("Home", false), searchKeeps("End", false), searchKeeps("ArrowDown", true), searchKeeps("ArrowDown", false), searchKeeps("Enter", false)]).toEqual([true, true, true, false, false]);
});

test("a provider with few models has no search box and no binding", () => {
  const { look } = model({ pointed: "small" });
  expect([look?.search, look?.menu?.bind]).toEqual([undefined, undefined]);
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
