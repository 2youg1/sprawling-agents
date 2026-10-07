// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The field's wiring as client/spec/Views/Parts.lean §7-2 states it:
// the label names the box, `aria-invalid` is exactly "the city refused
// it", `aria-describedby` is the page's description followed by this
// field's one line, an error replaces the help rather than joining it,
// and a box nobody may change puts back what the person typed. These
// hold for any look that spreads the bags, which is why the test
// imports no look.

import { describe, expect, test } from "bun:test";

import { lookOf, type FieldProps, type Typed } from "./field";

function typed(value: string): Typed {
  return { currentTarget: { value } };
}

function recorded(props: Omit<FieldProps, "onInput">): { readonly props: FieldProps; readonly seen: string[] } {
  const seen: string[] = [];
  return {
    props: {
      ...props,
      onInput: (value) => {
        seen.push(value);
      },
    },
    seen,
  };
}

describe("field wiring", () => {
  test("the label points at the box, and a bare box describes nothing and is valid", () => {
    const look = lookOf(recorded({ label: "Base URL", value: "" }).props, "f1");
    expect({
      for: look.labelWire.for,
      id: look.box.id,
      invalid: look.box["aria-invalid"],
      described: look.box["aria-describedby"],
      note: look.note,
    }).toEqual({ for: "f1-box", id: "f1-box", invalid: false, described: undefined, note: undefined });
  });

  test("help is the one line, read after the page's own description", () => {
    const look = lookOf(recorded({ label: "Base URL", value: "", help: "the host", describedBy: "page-7" }).props, "f1");
    expect([look.box["aria-describedby"], look.note]).toEqual([
      "page-7 f1-note",
      { kind: "help", text: "the host", wire: { id: "f1-note", role: undefined } },
    ]);
  });

  test("an error replaces the help, is an alert, and marks the box invalid", () => {
    const look = lookOf(recorded({ label: "Base URL", value: "x", help: "the host", error: "no such host" }).props, "f1");
    expect({ refused: look.refused, invalid: look.box["aria-invalid"], described: look.box["aria-describedby"], note: look.note }).toEqual({
      refused: true,
      invalid: true,
      described: "f1-note",
      note: { kind: "error", text: "no such host", wire: { id: "f1-note", role: "alert" } },
    });
  });

  test("an edit lands in an open box and is put back in a disabled one", () => {
    const open = recorded({ label: "Key", value: "old" });
    const event = typed("new");
    lookOf(open.props, "f1").box.oninput(event);
    expect([open.seen, event.currentTarget.value]).toEqual([["new"], "new"]);

    const shut = recorded({ label: "Key", value: "old", disabled: true });
    const refused = typed("new");
    const look = lookOf(shut.props, "f2");
    look.box.oninput(refused);
    expect([shut.seen, refused.currentTarget.value, look.box["aria-disabled"]]).toEqual([[], "old", true]);
  });

  test("a number asks for digits, and the other kinds ask for nothing", () => {
    const number = lookOf(recorded({ label: "Window", value: "1", kind: "number", step: 1024 }).props, "f1").box;
    const url = lookOf(recorded({ label: "Base URL", value: "", kind: "url" }).props, "f1").box;
    expect([number.type, number.inputmode, number.step, url.type, url.inputmode]).toEqual(["number", "numeric", 1024, "url", undefined]);
  });
});
