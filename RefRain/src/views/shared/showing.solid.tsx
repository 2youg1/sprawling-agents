// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { say } from "../../core/lang";
import { SHOWINGS } from "../../core/results";
import type { Showing } from "../../core/results";
import { ui } from "../../ui";
import Segmented from "../parts/segmented.solid";
import { storeValue } from "../../core/solid_store";

export type PortProps = Record<never, never>;

export default function SharedShowing() {
  const { prefs, lang } = ui();
  const $lang = storeValue(lang);
  const { held } = prefs;
  const $held = storeValue(held);
  return (
    <>
      <Segmented
        label={say($lang(), "showing_label")}
        options={SHOWINGS.map((value) => ({ value, label: say($lang(), `showing_${value}`) }))}
        held={$held().showing}
        onPick={(showing: Showing) => {
          prefs.setShowing(showing);
        }}
      />
    </>
  );
}
