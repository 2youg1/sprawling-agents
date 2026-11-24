// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import Button from "../parts/button.solid";

export interface UncheckedProps {
  readonly onRecheck: () => void;
}

export default function Unchecked(props: UncheckedProps) {
  const lang = storeValue(ui().lang);
  return (
    <div class="flex flex-wrap items-center gap-snug">
      <span class="text-note text-text-faint">{say(lang(), "machine_unasked")}</span>
      <Button label={say(lang(), "machine_recheck")} tone="secondary" onPress={props.onRecheck} />
    </div>
  );
}
