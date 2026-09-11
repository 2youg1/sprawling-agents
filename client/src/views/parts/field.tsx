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

import { Show, createUniqueId } from "solid-js";

export interface FieldProps {
  // Already in the person's language.
  readonly label: string;
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
  readonly kind?: "text" | "password" | "number";
  readonly disabled?: boolean;
}

export function Field(props: FieldProps) {
  const box = createUniqueId();
  const note = createUniqueId();
  // One description at a time: the error replaces the help, so a screen
  // reader is not read both halves of a contradiction.
  const described = () => (props.help === undefined && props.error === undefined ? undefined : note);
  return (
    <div class="flex w-full min-w-0 flex-col gap-tight">
      <label class="text-note text-text-quiet" for={box}>
        {props.label}
      </label>
      <div
        class={`flex min-w-0 items-center gap-tight rounded-control border px-base py-snug ${
          props.error === undefined ? "border-g3 bg-g2" : "border-alert bg-g2"
        }`}
      >
        <Show when={props.prefix}>
          {(prefix) => <span class="shrink-0 font-mono text-note text-text-faint">{prefix()}</span>}
        </Show>
        <input
          id={box}
          type={props.kind ?? "text"}
          class={`min-w-0 flex-1 bg-transparent text-body text-text outline-none placeholder:text-text-disabled ${
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
