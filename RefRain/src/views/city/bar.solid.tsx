// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, Show } from "solid-js";
import { QUERIES } from "../../core/asking";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import { barLookOf } from "./bar";
import Look from "./bar.look.solid";

export default function CityBar() {
  const u = ui();
  const lang = storeValue(u.lang);
  const metrics = storeValue(u.conn.asking.ask(QUERIES.metrics));
  const cost = storeValue(u.conn.asking.ask(QUERIES.cost));
  const look = createMemo(() => {
    const m = metrics();
    const c = cost();
    return barLookOf(
      m !== undefined && "metrics" in m ? m.metrics : null,
      c !== undefined && "cost" in c ? c.cost.total : null,
      lang(),
    );
  });
  return <Show when={look()}>{(bar) => <Look {...bar()} />}</Show>;
}
