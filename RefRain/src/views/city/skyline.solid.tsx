// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo } from "solid-js";
import { heldWithin } from "../../core/belief/rooms";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Address, CityAnswer } from "../../wire";
import { cornerPower } from "./shape";
import { lookOf } from "./skyline";
import Look from "./skyline.look.solid";

export { MIN_WIDTH } from "./skyline";

export interface SkylineProps {
  readonly city: CityAnswer;
  readonly picked: Address | null;
  readonly onPick: (addr: Address) => void;
}

export default function Skyline(props: SkylineProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const power = cornerPower(document.documentElement);
  const look = createMemo(() =>
    lookOf(
      {
        city: props.city,
        picked: props.picked,
        runsIn: (addr) => heldWithin(belief(), addr),
        lang: lang(),
        power,
      },
      (addr) => props.onPick(addr),
    ),
  );
  return <Look {...look()} />;
}
