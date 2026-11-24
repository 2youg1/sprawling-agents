// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal, For, Show } from "solid-js";

import { ACTIONS, DEFAULTS, LABELS, keymap, spell, type Action } from "../../core/keys";
import { fill, say, type Key } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import Button from "../parts/button.solid";
import { chordOf, type ChordHands } from "./chord";
import ChordView from "./chord_view";

export default function KeysSection() {
  const lang = storeValue(ui().lang);
  const keys = keymap();
  const bound = storeValue(keys.bound);
  const clashes = storeValue(keys.conflicts);
  const [recording, setRecording] = createSignal<Action | null>(null);
  const said = (key: Key) => say(lang(), key);
  const hands: ChordHands = { listen: setRecording, bind: keys.bind };

  function KeyRow(props: { readonly action: Action }) {
    const spelled = createMemo(() => spell(bound()[props.action]));
    const conflicting = createMemo(() =>
      clashes()
        .find((clash) => clash.spelled === spelled())
        ?.actions.find((action) => action !== props.action),
    );
    const look = createMemo(() =>
      chordOf(
        {
          action: props.action,
          spelled: spelled(),
          listening: recording() === props.action,
          prompt: said("keys_press"),
          unbound: said("keys_unbound"),
          tip: said("keys_change"),
        },
        hands,
      ),
    );

    return (
      <li class="flex items-center gap-base border-b border-edge py-snug text-label last:border-b-0">
        <span class="min-w-0 flex-1 truncate text-text-quiet">{said(LABELS[props.action])}</span>
        <Show when={conflicting()} keyed>
          {(action) => (
            <span class="truncate text-note text-alert">
              {fill(said("keys_conflict"), { name: said(LABELS[action]) })}
            </span>
          )}
        </Show>
        <ChordView {...look()} />
        <Button
          {...(spelled() === spell(DEFAULTS[props.action]) ? { why: said("keys_at_default") } : {})}
          label={said("keys_reset")}
          tone="quiet"
          onPress={() => keys.reset(props.action)}
        />
      </li>
    );
  }

  return (
    <section class="border-t border-edge py-wide">
      <h2 class="mb-base text-label font-label text-text-quiet">{said("keys_title")}</h2>
      <ul class="flex flex-col">
        <For each={ACTIONS}>{(action) => <KeyRow action={action} />}</For>
      </ul>
      <Button label={said("keys_reset_all")} tone="quiet" onPress={() => keys.resetAll()} />
    </section>
  );
}
