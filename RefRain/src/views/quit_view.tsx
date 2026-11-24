// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, untrack } from "solid-js";
import { quitAsked } from "../core/quitting";
import { storeValue } from "../core/solid_store";
import { ui } from "../ui";
import { quitOf } from "./quit";
import QuitSheet from "./quit_sheet.solid";

export default function Quit() {
  const u = ui();
  const belief = storeValue(u.conn.belief);
  const asks = storeValue(quitAsked);
  const [open, setOpen] = createSignal(false);
  let seen = untrack(asks);
  createEffect(asks, (next) => {
    if (next === seen) return;
    seen = next;
    setOpen(true);
  });
  const look = createMemo(() => quitOf({ runs: belief().live.length, here: u.origin }));
  return (
    <QuitSheet
      look={look()}
      seat="modal"
      open={open()}
      onCancel={() => {
        setOpen(false);
      }}
      onClose={(mode) => {
        setOpen(false);
        u.conn.closeCity(mode);
      }}
    />
  );
}
