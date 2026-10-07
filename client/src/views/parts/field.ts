// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the one labelled box decides before anything is drawn: the ids
// that tie the label, the box and the line under it together, which
// description a screen reader is given, and the gate that keeps an edit
// from landing in a box nobody may change. The look
// (`field.look.svelte`, or any other look that takes `FieldLook`)
// spreads each bag on the element it names and writes no ARIA and no
// handler of its own (client/spec/Views/Parts.lean §7-2).
//
// A form owes a box four things in one place: the name of the value,
// the line that helps before the attempt, the line that explains after
// a failed one, and the fixed parts of the value the person should not
// have to type. The error sits under the box rather than in a corner of
// the screen, because the field is where the correction is made.

export interface FieldProps {
  // Already in the person's language.
  readonly label: string;
  // Where the label is drawn. `hidden` keeps it as the box's accessible
  // name and nothing else, for a row or a cell whose column already
  // carries the word.
  readonly labelling?: "above" | "hidden";
  readonly value: string;
  readonly onInput: (value: string) => void;
  readonly help?: string;
  // Present means the value was refused, and says what to do next.
  readonly error?: string;
  // Parts of the value the page knows: a scheme, a path, a unit.
  readonly prefix?: string;
  readonly suffix?: string;
  readonly placeholder?: string;
  // A value read character by character - a key, an id, a path - is set
  // in the mono face so a person can check it.
  readonly mono?: boolean;
  // `number` also asks the on-screen keyboard for digits and gives the
  // arrow keys a step; `url` lets the browser refuse a value this form
  // would otherwise send to the city to be refused there.
  readonly kind?: "text" | "password" | "number" | "url";
  // How far one arrow key moves a number.
  readonly step?: number;
  // What a valid value looks like, for the browser's first pass.
  readonly pattern?: string;
  // An explanation the page already draws elsewhere. It is read out
  // before this field's own help, never instead of it.
  readonly describedBy?: string;
  // Present means the value may not be changed. The box stays in the Tab
  // sequence and keeps `aria-disabled`, because a `disabled` element is
  // skipped by the keyboard and its reason is never read out; the input
  // gate in the box's bag is what keeps an edit from landing.
  readonly disabled?: boolean;
}

// The part of an input event the gate reads and, when it refuses,
// writes back: the box the event came from.
export interface Typed {
  readonly currentTarget: { value: string };
}

// Spread on the `<label>`.
export interface LabelWire {
  readonly for: string;
}

// Spread on the `<input>`.
export interface BoxWire {
  readonly id: string;
  readonly type: "text" | "password" | "number" | "url";
  readonly inputmode: "numeric" | undefined;
  readonly step: number | undefined;
  readonly pattern: string | undefined;
  readonly value: string;
  readonly placeholder: string | undefined;
  readonly "aria-disabled": boolean;
  readonly "aria-invalid": boolean;
  readonly "aria-describedby": string | undefined;
  readonly oninput: (event: Typed) => void;
}

// The line under the box: the city's refusal, or the help when there is
// none. One at a time, so a screen reader is not read both halves of a
// contradiction.
export interface NoteLook {
  readonly kind: "error" | "help";
  readonly text: string;
  // Spread on the line: its id, and `role="alert"` for a refusal.
  readonly wire: { readonly id: string; readonly role: "alert" | undefined };
}

export interface FieldLook {
  readonly label: string;
  readonly labelling: "above" | "hidden";
  readonly prefix: string | undefined;
  readonly suffix: string | undefined;
  readonly mono: boolean;
  readonly disabled: boolean;
  // The city refused the value: the border is red at once.
  readonly refused: boolean;
  readonly labelWire: LabelWire;
  readonly box: BoxWire;
  readonly note: NoteLook | undefined;
}

// The page's own description first, then this field's line: the error
// replaces the help, and the page's `describedBy` survives either way.
export function describedOf(props: FieldProps, note: string): string | undefined {
  const own = props.help === undefined && props.error === undefined ? [] : [note];
  const ids = [...(props.describedBy === undefined ? [] : [props.describedBy]), ...own];
  return ids.length === 0 ? undefined : ids.join(" ");
}

function noteOf(props: FieldProps, id: string): NoteLook | undefined {
  if (props.error !== undefined) return { kind: "error", text: props.error, wire: { id, role: "alert" } };
  if (props.help !== undefined) return { kind: "help", text: props.help, wire: { id, role: undefined } };
  return undefined;
}

// `uid` is the seat's own id; the box and the line take it as a prefix.
export function lookOf(props: FieldProps, uid: string): FieldLook {
  const box = `${uid}-box`;
  const note = `${uid}-note`;
  const kind = props.kind ?? "text";
  const disabled = props.disabled ?? false;
  return {
    label: props.label,
    labelling: props.labelling ?? "above",
    prefix: props.prefix,
    suffix: props.suffix,
    mono: props.mono ?? false,
    disabled,
    refused: props.error !== undefined,
    labelWire: { for: box },
    box: {
      id: box,
      type: kind,
      inputmode: kind === "number" ? "numeric" : undefined,
      step: props.step,
      pattern: props.pattern,
      value: props.value,
      placeholder: props.placeholder,
      "aria-disabled": disabled,
      "aria-invalid": props.error !== undefined,
      "aria-describedby": describedOf(props, note),
      oninput: (event) => {
        if (disabled) {
          event.currentTarget.value = props.value;
          return;
        }
        props.onInput(event.currentTarget.value);
      },
    },
    note: noteOf(props, note),
  };
}
