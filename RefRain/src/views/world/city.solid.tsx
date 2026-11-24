// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, Match, onSettled, Switch } from "solid-js";
import { readAnswer } from "../../core/answered";
import { QUERIES } from "../../core/asking";
import { heldWithin } from "../../core/belief/rooms";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Address, CityAnswer } from "../../wire";
import Skyline, { MIN_WIDTH } from "../city/skyline.solid";
import Unanswered from "../parts/unanswered.solid";

function CityViewport(props: {
  readonly city: CityAnswer;
  readonly onPick: (at: Address) => void;
}) {
  let scroller: HTMLDivElement | undefined;
  onSettled(() => {
    if (scroller) scroller.scrollLeft = (scroller.scrollWidth - scroller.clientWidth) / 2;
  });
  return (
    <div
      class="min-h-0 overflow-x-auto"
      ref={(node) => {
        scroller = node;
      }}
    >
      <div style={{ "min-width": `${MIN_WIDTH}px` }}>
        <Skyline city={props.city} picked={null} onPick={props.onPick} />
      </div>
    </div>
  );
}

export default function WorldCity() {
  const u = ui();
  const belief = storeValue(u.conn.belief);
  const city = storeValue(u.conn.asking.ask(QUERIES.city));
  const read = createMemo(() =>
    readAnswer(city(), (held) => ("city" in held ? held.city : undefined)),
  );
  const unavailable = () => {
    const result = read();
    return result.kind === "unavailable" ? result : null;
  };
  const held = () => {
    const result = read();
    return result.kind === "held" ? result.value : null;
  };

  function enter(building: Address): void {
    const newest = heldWithin(belief(), building)
      .filter((run) => run.addr !== null)
      .sort((a, b) => (a.started ?? 0) - (b.started ?? 0))
      .at(-1)?.addr;
    u.go({ kind: "talk", address: newest ?? building });
  }

  return (
    <Switch>
      <Match when={unavailable()}>
        {(lost) => <Unanswered query={lost().query} asked={QUERIES.city} />}
      </Match>
      <Match when={held()}>{(answer) => <CityViewport city={answer()} onPick={enter} />}</Match>
    </Switch>
  );
}
