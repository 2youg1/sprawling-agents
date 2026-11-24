// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createUniqueId, For, Show } from "solid-js";
import { fill, say } from "../core/lang";
import { storeValue } from "../core/solid_store";
import { ui } from "../ui";
import Button from "./parts/button.solid";
import ModalSheet from "./parts/modal_sheet";
import type { QuitSheetProps } from "./quit";

export default function QuitSheet(props: QuitSheetProps) {
  const uid = createUniqueId();
  const lang = storeValue(ui().lang);
  return (
    <ModalSheet
      open={props.open}
      stands={props.seat === "specimen" ? "in-flow" : "centre"}
      title={uid + "-title"}
      description={uid + "-about"}
      onCancel={props.onCancel}
    >
      <h2 id={uid + "-title"} class="text-heading text-text">
        {say(lang(), "quit_title")}
      </h2>
      <p id={uid + "-about"} class="text-note text-text-quiet">
        {say(lang(), props.look.about)}
      </p>
      <Show when={props.look.runs > 0}>
        <p class="text-note text-text">
          {fill(say(lang(), "quit_runs"), { n: String(props.look.runs) })}
        </p>
      </Show>
      <div class="flex flex-wrap items-center justify-end gap-snug">
        <Button label={say(lang(), "quit_cancel")} tone="secondary" onPress={props.onCancel} />
        <For each={props.look.answers} keyed={(answer) => answer.mode}>
          {(answer) => (
            <Button
              label={say(lang(), answer().label)}
              tone={answer().tone}
              onPress={() => props.onClose(answer().mode)}
            />
          )}
        </For>
      </div>
    </ModalSheet>
  );
}
