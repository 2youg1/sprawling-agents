// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One labelled box a person types in, with everything a form owes it in
// the same place: the name of the value, the line that helps before the
// attempt, the line that explains after a failed one, and the fixed
// parts of the value the person should not have to type.
//
// The error sits under the box rather than in a corner of the screen,
// because the field is where the correction is made.
//
// **Two authorities decide the red border, and they say different
// things.** `error` is the city's answer, so it is drawn the moment the
// page is told; `:user-invalid` is the browser's own reading of `type`
// and `pattern`, and it holds off until the person has left the box, so
// a URL is never red while it is half typed.
//
// The box carries no `outline` rule of its own. The focus ring is one
// declaration in `theme.css`, and a box that hid it was the reason a
// keyboard user could not see where they were on the settings page.

import { Show, createUniqueId } from "solid-js";

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
  readonly disabled?: boolean;
}

export function Field(props: FieldProps) {
  const box = createUniqueId();
  const note = createUniqueId();
  // One description at a time from this field: the error replaces the
  // help, so a screen reader is not read both halves of a contradiction.
  const described = () => {
    const own = props.help === undefined && props.error === undefined ? undefined : note;
    const ids = [props.describedBy, own].filter((id): id is string => id !== undefined);
    return ids.length === 0 ? undefined : ids.join(" ");
  };
  const kind = () => props.kind ?? "text";
  return (
    <div class="flex w-full min-w-0 flex-col gap-tight">
      <label class={`text-note text-text-quiet ${props.labelling === "hidden" ? "sr-only" : ""}`} for={box}>
        {props.label}
      </label>
      <div
        class={`flex min-w-0 items-center gap-tight rounded-control border bg-g2 px-base py-snug has-[:user-invalid]:border-alert ${
          props.error === undefined ? "border-g3" : "border-alert"
        }`}
      >
        <Show when={props.prefix}>
          {(prefix) => <span class="shrink-0 font-mono text-note text-text-faint">{prefix()}</span>}
        </Show>
        <input
          id={box}
          type={kind()}
          inputmode={kind() === "number" ? "numeric" : undefined}
          step={props.step}
          pattern={props.pattern}
          class={`min-w-0 flex-1 bg-transparent text-body text-text placeholder:text-text-disabled ${
            props.mono === true ? "font-mono" : ""
          }`}
          value={props.value}
          placeholder={props.placeholder}
          disabled={props.disabled === true}
          aria-invalid={props.error !== undefined}
          aria-describedby={described()}
          onInput={(event) => {
            props.onInput(event.currentTarget.value);
          }}
        />
        <Show when={props.suffix}>
          {(suffix) => <span class="shrink-0 font-mono text-note text-text-faint">{suffix()}</span>}
        </Show>
      </div>
      <Show when={props.error} fallback={<Show when={props.help}>{(help) => <p id={note} class="text-note text-text-faint">{help()}</p>}</Show>}>
        {(error) => (
          <p id={note} class="text-note text-alert" role="alert">
            {error()}
          </p>
        )}
      </Show>
    </div>
  );
}
