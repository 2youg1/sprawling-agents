// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo } from "solid-js";
import { heldWithin } from "../../core/belief/rooms";
import { ui } from "../../ui";
import type { Address, CityAnswer } from "../../wire";
import { panelLookOf } from "./panel";
import Look from "./panel.look.solid";
interface PanelProps {
  readonly addr: Address;
  readonly city: CityAnswer;
  readonly onClose: () => void;
}
import { storeValue } from "../../core/solid_store";

export type PortProps = PanelProps;

export default function CityPanel(props: PanelProps) {
  const u = ui();
  const { lang } = u;
  const $lang = storeValue(lang);
  const belief = u.conn.belief;
  const $belief = storeValue(belief);
  const look = createMemo(() =>
    panelLookOf(
      {
        addr: props.addr,
        city: props.city,
        held: heldWithin($belief(), props.addr),
        lang: $lang(),
      },
      () => {
        props.onClose();
      },
    ),
  );
  return (
    <>
      <Look {...look()} />
    </>
  );
}
