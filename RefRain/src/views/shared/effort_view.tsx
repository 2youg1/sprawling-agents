// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, Show } from "solid-js";

import { QUERIES } from "../../core/asking";
import { fill, say, type Key } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { readable } from "../../core/store";
import { ui } from "../../ui";

const NO_CONFIG = readable(undefined);

export default function EffortSection() {
  const u = ui();
  const lang = storeValue(u.lang);
  const city = storeValue(u.conn.asking.ask(QUERIES.city));
  const first = createMemo(() => {
    const held = city();
    return held !== undefined && "city" in held
      ? held.city.buildings
          .map((item) => item.addr)
          .sort((left, right) => left.localeCompare(right))
          .at(0)
      : undefined;
  });
  const config = storeValue(() => {
    const addr = first();
    return addr === undefined ? NO_CONFIG : u.conn.asking.ask({ config: { addr } });
  });
  const word = (key: Key) => say(lang(), key);
  const standing = createMemo(() => {
    const answer = config();
    const addr = first();
    if (addr === undefined || answer === undefined || !("config" in answer)) return null;
    const settled = answer.config.effort;
    return settled == null
      ? fill(word("setup_effort_unstated"), { addr })
      : fill(word("setup_effort_standing"), {
          addr,
          effort: settled.effort,
          from: word(`setup_effort_from_${settled.from}`),
        });
  });

  return (
    <div class="flex flex-col gap-base">
      <p class="text-note text-text-quiet">{word("setup_effort")}</p>
      <p class="max-w-measure text-note leading-read text-text-faint">
        {word("setup_effort_essay")}
      </p>
      <Show when={standing()}>
        {(value) => <p class="max-w-measure text-note text-text">{value()}</p>}
      </Show>
      <p class="max-w-measure text-note text-text-quiet">{word("setup_effort_city")}</p>
      <p class="max-w-measure text-note text-text-quiet">{word("setup_effort_where")}</p>
    </div>
  );
}
