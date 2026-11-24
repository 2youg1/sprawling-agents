// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Show, createMemo } from "solid-js";
import { reveal } from "../../core/commands";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import IconButton from "./icon_button.solid";
import { lookOf, type PathProps } from "./path";

const TEXT = "min-w-0 truncate font-mono text-note text-text-quiet";

export default function Path(props: PathProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const look = createMemo(() =>
    lookOf(
      props,
      {
        reveal: say(lang(), "path_reveal"),
        inert: say(lang(), "path_reveal_inert"),
      },
      (address) => {
        u.send(reveal(address));
      },
    ),
  );
  return (
    <span class="inline-flex min-w-0 max-w-full items-baseline gap-tight">
      <Show when={look().open} fallback={<span class={TEXT}>{look().shown}</span>}>
        {(open) => (
          <button
            data-control=""
            type="button"
            onClick={open().onclick}
            class={`${TEXT} text-left underline decoration-edge underline-offset-2 transition-colors ease-leave hover:text-text hover:ease-arrive still:transition-none`}
          >
            {look().shown}
          </button>
        )}
      </Show>
      <IconButton {...look().reveal} />
    </span>
  );
}
