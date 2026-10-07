<script lang="ts" module>
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
// declaration in `theme/base.css`, and a box that hid it was the reason a
// keyboard user could not see where they were on the settings page.

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
  // gate below is what keeps an edit from landing.
  readonly disabled?: boolean;
}
</script>

<script lang="ts">
  const {
    label,
    labelling = "above",
    value,
    onInput,
    help,
    error,
    prefix,
    suffix,
    placeholder,
    mono = false,
    kind = "text",
    step,
    pattern,
    describedBy,
    disabled = false,
  }: FieldProps = $props();

  const uid = $props.id();
  const box = `${uid}-box`;
  const note = `${uid}-note`;

  // One description at a time from this field: the error replaces the
  // help, so a screen reader is not read both halves of a contradiction.
  // The page's own `describedBy` comes first and survives either way.
  const described = $derived.by((): string | undefined => {
    const own = help === undefined && error === undefined ? undefined : note;
    const ids: string[] = [];
    if (describedBy !== undefined) ids.push(describedBy);
    if (own !== undefined) ids.push(own);
    return ids.length === 0 ? undefined : ids.join(" ");
  });
</script>

<div class="flex w-full min-w-0 flex-col gap-tight">
  <label class="text-note text-text-quiet {labelling === 'hidden' ? 'sr-only' : ''}" for={box}>
    {label}
  </label>
  <!-- The city's refusal reddens the border at once; the browser's own
  reading waits for the blur, and a box nobody may edit is never graded
  on what it holds, so `:user-invalid` is switched off while disabled. -->
  <div
    class="flex h-control min-w-0 items-center gap-tight rounded-control border bg-raised px-base {error ===
    undefined
      ? 'border-edge-input'
      : 'border-alert'} {disabled ? '' : 'has-[:user-invalid]:border-alert'}"
  >
    {#if prefix !== undefined}
      <span class="shrink-0 font-mono text-note text-text-faint">{prefix}</span>
    {/if}
    <input
      id={box}
      type={kind}
      inputmode={kind === "number" ? "numeric" : undefined}
      {step}
      {pattern}
      class="min-w-0 flex-1 bg-transparent text-body {disabled
        ? 'aria-disabled:text-text-disabled'
        : 'text-text'} placeholder:text-text-faint {mono ? 'font-mono' : ''}"
      {value}
      {placeholder}
      aria-disabled={disabled}
      aria-invalid={error !== undefined}
      aria-describedby={described}
      oninput={(event) => {
        if (disabled) {
          event.currentTarget.value = value;
          return;
        }
        onInput(event.currentTarget.value);
      }}
    />
    {#if suffix !== undefined}
      <span class="shrink-0 font-mono text-note text-text-faint">{suffix}</span>
    {/if}
  </div>
  {#if error !== undefined}
    <p id={note} class="text-note text-alert" role="alert">{error}</p>
  {:else if help !== undefined}
    <p id={note} class="text-note text-text-faint">{help}</p>
  {/if}
</div>
