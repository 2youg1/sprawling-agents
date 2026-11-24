// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createUniqueId, For } from "solid-js";
import { ACTIONS, keymap, LABELS, marks } from "../../core/keys";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import Button from "./button.solid";
import type { Seat, Words } from "./kbd";
import { KbdMarks } from "./kbd.look";
import ModalSheet from "./modal_sheet";

export interface CheatsheetProps {
  readonly seat: Seat;
  readonly onClose: () => void;
}
export default function Cheatsheet(props: CheatsheetProps) {
  const lang = storeValue(ui().lang);
  const keys = keymap();
  const bound = storeValue(keys.bound);
  const rows = createMemo(() =>
    ACTIONS.map((action) => ({
      key: action,
      label: say(lang(), LABELS[action]),
      marks: marks(bound()[action], keys.platform),
    })).filter((row) => row.marks.length > 0),
  );
  return (
    <CheatsheetLook
      {...props}
      title={say(lang(), "keys_title")}
      dismiss={say(lang(), "dismiss")}
      where={say(lang(), "keys_where")}
      rows={rows()}
    />
  );
}

export function CheatsheetLook(props: CheatsheetProps & Words) {
  const uid = createUniqueId();
  return (
    <ModalSheet
      open
      stands={props.seat === "modal" ? "below-top" : "in-flow"}
      title={uid + "-title"}
      description={uid + "-where"}
      onCancel={props.onClose}
    >
      <div class="flex items-baseline justify-between">
        <h2 id={uid + "-title"} class="text-heading font-heading text-text">
          {props.title}
        </h2>
        <Button label={props.dismiss} tone="quiet" onPress={props.onClose} />
      </div>
      <ul class="flex flex-col">
        <For each={props.rows} keyed={(row) => row.key}>
          {(row) => (
            <li class="flex items-center justify-between gap-base py-tight text-label">
              <span class="truncate text-text-quiet">{row().label}</span>
              <KbdMarks marks={row().marks} />
            </li>
          )}
        </For>
      </ul>
      <p id={uid + "-where"} class="text-note text-text-faint">
        {props.where}
      </p>
    </ModalSheet>
  );
}
