// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, Match, Switch } from "solid-js";
import { NO_TAIL } from "../../core/live_output";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Address, Call, RoundsAnswer } from "../../wire";
import Diff from "./diff.solid";
import File from "./file.solid";
import { pictureOf, readFrom, readingOf } from "./reading";
import Shot from "./shot.solid";
import Terminal from "./terminal.solid";

export interface InspectorCallProps {
  readonly call: Call;
  readonly rounds: RoundsAnswer;
  readonly talk: Address;
}

export default function InspectorCall(props: InspectorCallProps) {
  const live = storeValue(ui().conn.live);
  const reading = createMemo(() => readingOf(props.call));
  const read = createMemo(() => (reading() === "file" ? readFrom(props.call) : null));
  const picture = createMemo(() => (reading() === "shot" ? pictureOf(props.call) : null));
  return (
    <Switch fallback={<Terminal call={props.call} tail={live()[props.rounds.run] ?? NO_TAIL} />}>
      <Match when={reading() === "diff"}>
        <Diff call={props.call} rounds={props.rounds} talk={props.talk} />
      </Match>
      <Match when={read()}>{(held) => <File read={held()} />}</Match>
      <Match when={picture()}>{(held) => <Shot picture={held()} />}</Match>
    </Switch>
  );
}
