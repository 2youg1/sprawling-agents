// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { createMemo, createUniqueId, For, Show } from "solid-js";
import { keymap, marks } from "../../core/keys";
import { pressedOf } from "../../core/press";
import { storeValue } from "../../core/solid_store";
import Button from "./button.solid";
import { answering, GLYPH, KEY, ordered, type Choice, type DecideKind } from "./decide_model";
import Glyph from "./glyph.solid";
import { KbdMarks } from "./kbd.look";

export interface DecideViewProps {
  readonly kind: DecideKind;
  readonly asker: string;
  readonly at: string;
  readonly choices: readonly Choice[];
  readonly children: JSX.Element;
}

export default function Decide(props: DecideViewProps) {
  const keys = keymap();
  const bound = storeValue(keys.bound);
  const id = createUniqueId();
  const choices = createMemo(() => ordered(props.choices));
  return (
    <div
      class="asks flex min-w-0 flex-col gap-snug rounded-card py-base pr-pane focus-visible:wash"
      role="group"
      tabindex={-1}
      aria-labelledby={id}
      data-entry=""
      data-decide={props.kind}
      onKeyDown={(event) => {
        if (event.isComposing) return;
        const chosen = answering(choices(), keys.acting(pressedOf(event)));
        if (chosen === undefined) return;
        event.preventDefault();
        event.stopPropagation();
        chosen.onPress();
      }}
    >
      <div class="flex min-w-0 items-center gap-snug text-note">
        <Glyph name={GLYPH[props.kind]} size="sm" class="shrink-0 text-alert" />
        <span id={id} class="min-w-0 flex-1 truncate text-text-quiet">
          {props.asker}
        </span>
        <span class="figure shrink-0 text-text-faint">{props.at}</span>
        <kbd class="entry-n" aria-hidden="true" />
      </div>
      <div class="min-w-0">{props.children}</div>
      <Show when={choices().length > 0}>
        <div class="flex flex-wrap items-center justify-end gap-snug">
          <For each={choices()} keyed={(choice) => choice.answer}>
            {(choice) => (
              <span class="inline-flex items-center gap-tight">
                <Button
                  label={choice().label}
                  tone={choice().answer === "yes" ? "primary" : "quiet"}
                  {...(choice().why === undefined ? {} : { why: choice().why ?? "" })}
                  onPress={choice().onPress}
                />
                <span aria-hidden="true">
                  <KbdMarks marks={marks(bound()[KEY[choice().answer]], keys.platform)} />
                </span>
              </span>
            )}
          </For>
        </div>
      </Show>
    </div>
  );
}
