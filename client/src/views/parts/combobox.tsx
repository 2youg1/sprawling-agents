// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A choice among many, searched rather than scrolled: the control a
// provider's model list needs, where a native select puts two hundred
// rows behind one arrow.
//
// The keyboard owns it: the trigger opens, typing filters, the arrows
// move, Enter picks, Escape closes and gives the focus back to the
// trigger it came from.

import { For, Show, createMemo, createSignal } from "solid-js";

export interface Choice {
  readonly value: string;
  // Already in the person's language, or an identifier they typed.
  readonly label: string;
  // The second line: a provider, a price, a context window.
  readonly note?: string;
}

export interface ComboboxProps {
  // The accessible name of the control.
  readonly label: string;
  // What the trigger says while nothing is chosen, and what the search
  // box says while it is empty.
  readonly placeholder: string;
  // What stands in the list when the search matches nothing.
  readonly empty: string;
  readonly choices: readonly Choice[];
  readonly value: string | null;
  readonly onPick: (value: string) => void;
}

export function Combobox(props: ComboboxProps) {
  const [open, setOpen] = createSignal(false);
  const [query, setQuery] = createSignal("");
  const [at, setAt] = createSignal(0);
  const [trigger, setTrigger] = createSignal<HTMLButtonElement>();

  const matches = createMemo(() => {
    const needle = query().trim().toLowerCase();
    if (needle === "") return props.choices;
    return props.choices.filter((choice) =>
      `${choice.label} ${choice.note ?? ""} ${choice.value}`.toLowerCase().includes(needle),
    );
  });
  // The cursor is clamped where it is read rather than reset where the
  // list changes: a filter that shortens the list must not be able to
  // leave the cursor pointing past its end.
  const cursor = () => Math.min(at(), Math.max(matches().length - 1, 0));
  const chosen = () => props.choices.find((choice) => choice.value === props.value);

  const shut = () => {
    setOpen(false);
    setQuery("");
    setAt(0);
    trigger()?.focus();
  };
  const take = (choice: Choice | undefined) => {
    if (choice === undefined) return;
    props.onPick(choice.value);
    shut();
  };

  return (
    <div class="relative flex w-full min-w-0 flex-col gap-tight">
      <button
        ref={setTrigger}
        type="button"
        class="flex w-full min-w-0 items-center justify-between gap-snug rounded-control border border-g3 bg-g2 px-base py-snug text-body text-text hover:bg-g3"
        aria-label={props.label}
        aria-haspopup="listbox"
        aria-expanded={open()}
        onClick={() => {
          setOpen(!open());
        }}
      >
        <span class="truncate">
          <Show when={chosen()} fallback={<span class="text-text-disabled">{props.placeholder}</span>}>
            {(choice) => choice().label}
          </Show>
        </span>
      </button>
      <Show when={open()}>
        <div class="absolute top-full left-0 z-10 mt-tight flex w-full flex-col rounded-panel border border-g3 bg-g1 p-tight shadow-composer transition-[opacity,transform] duration-200 ease-[cubic-bezier(0.2,0,0,1)] motion-reduce:transition-none">
          <input
            ref={(input) => {
              requestAnimationFrame(() => {
                input.focus();
              });
            }}
            class="mb-tight w-full rounded-control bg-g2 px-base py-snug text-body text-text outline-none placeholder:text-text-disabled"
            placeholder={props.placeholder}
            aria-label={props.label}
            value={query()}
            onInput={(event) => {
              setQuery(event.currentTarget.value);
              setAt(0);
            }}
            onKeyDown={(event) => {
              switch (event.key) {
                case "ArrowDown":
                  event.preventDefault();
                  setAt(Math.min(cursor() + 1, matches().length - 1));
                  return;
                case "ArrowUp":
                  event.preventDefault();
                  setAt(Math.max(cursor() - 1, 0));
                  return;
                case "Enter":
                  event.preventDefault();
                  take(matches()[cursor()]);
                  return;
                case "Escape":
                  event.preventDefault();
                  shut();
                  return;
                default:
                  return;
              }
            }}
          />
          <ul class="max-h-output overflow-y-auto" role="listbox" aria-label={props.label}>
            <Show
              when={matches().length > 0}
              fallback={<li class="px-base py-snug text-note text-text-faint">{props.empty}</li>}
            >
              <For each={matches()}>
                {(choice, index) => (
                  <li
                    role="option"
                    aria-selected={choice.value === props.value}
                    class={`flex cursor-pointer items-baseline justify-between gap-snug rounded-control px-base py-snug text-body ${
                      index() === cursor() ? "bg-g3 text-text" : "text-text-quiet"
                    }`}
                    onMouseEnter={() => {
                      setAt(index());
                    }}
                    onClick={() => {
                      take(choice);
                    }}
                  >
                    <span class="truncate">{choice.label}</span>
                    <Show when={choice.note}>
                      {(note) => <span class="shrink-0 text-note text-text-faint">{note()}</span>}
                    </Show>
                  </li>
                )}
              </For>
            </Show>
          </ul>
        </div>
      </Show>
    </div>
  );
}
