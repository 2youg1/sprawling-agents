// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal, Match, onSettled, Switch } from "solid-js";

import { forgetDevice } from "../../core/access";
import { readAnswer } from "../../core/answered";
import { QUERIES } from "../../core/asking";
import { kept } from "../../core/local/device";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { DeviceId } from "../../wire";
import Unanswered from "../parts/unanswered.solid";
import DeviceList from "./devices_list_view";

export default function Devices() {
  const u = ui();
  const asked = storeValue(u.conn.asking.ask(QUERIES.devices));
  const read = createMemo(() =>
    readAnswer(asked(), (answer) => ("devices" in answer ? answer.devices.devices : undefined)),
  );
  const devices = () => {
    const answer = read();
    return answer.kind === "held" ? answer.value : undefined;
  };
  const unavailable = () => {
    const answer = read();
    return answer.kind === "unavailable" ? answer.query : undefined;
  };
  const [here, setHere] = createSignal<string | null>(null);
  const now = u.now();
  const local = !u.origin.startsWith("https:");

  onSettled(() => {
    let alive = true;
    void kept().then((device) => {
      if (alive) setHere(device?.id ?? null);
    });
    return () => {
      alive = false;
    };
  });

  function forget(device: DeviceId): void {
    if (u.send(forgetDevice(device))) u.conn.asking.refresh(QUERIES.devices);
  }

  return (
    <Switch>
      <Match when={devices()}>
        {(rows) => (
          <DeviceList devices={rows()} here={here()} now={now} local={local} onForget={forget} />
        )}
      </Match>
      <Match when={unavailable()}>
        {(query) => <Unanswered query={query()} asked={QUERIES.devices} />}
      </Match>
    </Switch>
  );
}
