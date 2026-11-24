// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, Show } from "solid-js";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { RunId } from "../../wire";
import { lastCheckpointIn } from "../checkpoints";
import Produced from "../talk/produced.solid";

export interface CityProducedProps {
  readonly run: RunId;
}

export default function CityProduced(props: CityProducedProps) {
  const asking = ui().conn.asking;
  const asked = storeValue(() => asking.ask({ rounds: { run: props.run } }));
  const range = createMemo(() => {
    const held = asked();
    if (held === undefined || !("rounds" in held)) return null;
    const base = held.rounds.opened_at ?? null;
    const head = lastCheckpointIn(held.rounds.turns);
    return base === null ? null : { base, head: head === base ? null : head };
  });
  return <Show when={range()}>{(span) => <Produced base={span().base} head={span().head} />}</Show>;
}
