// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { ui } from "../../ui";
import { lookOf } from "./ask_mayor";
import Look from "./ask_mayor.look.solid";
import { storeValue } from "../../core/solid_store";

export type PortProps = Record<never, never>;

export default function CityAskMayor() {
  const { lang } = ui();
  const $lang = storeValue(lang);
  return (
    <>
      <Look {...lookOf($lang())} />
    </>
  );
}
