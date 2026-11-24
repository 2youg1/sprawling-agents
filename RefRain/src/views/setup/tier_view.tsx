// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { say } from "../../core/lang";
import { SESSION_TIERS } from "../../core/prefs";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import Segmented from "../parts/segmented.solid";
import Card from "./card_view";

export default function Tier() {
  const { lang, prefs } = ui();
  const language = storeValue(lang);
  const held = storeValue(prefs.held);
  return (
    <Card title={say(language(), "appearance_tier")} note={say(language(), "appearance_tier_note")}>
      <Segmented
        label={say(language(), "appearance_tier")}
        options={SESSION_TIERS.map((tier) => ({
          value: tier,
          label: say(language(), `tier_${tier}`),
        }))}
        held={held().tier}
        onPick={prefs.setTier}
      />
    </Card>
  );
}
