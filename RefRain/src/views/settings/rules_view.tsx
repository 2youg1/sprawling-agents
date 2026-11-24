// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createSignal } from "solid-js";

import { putRules } from "../../core/commands";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import { DocumentDraft } from "../setup/governed_view";
import { HALL } from "../shared/buildings";
import BuildingPicker from "./building_picker";
import { rulesAt } from "./files";

export default function Rules() {
  const lang = storeValue(ui().lang);
  const [which, setWhich] = createSignal(HALL);
  return (
    <DocumentDraft
      which={which()}
      path={rulesAt}
      command={(at, base, text) => {
        const frame = putRules(at, base, text);
        return "put_rules" in frame ? { frame, idem: frame.put_rules.idem } : undefined;
      }}
      title={say(lang(), "rules_title")}
      note={say(lang(), "rules_note")}
      field="rules_title"
      save="saving_save"
    >
      <BuildingPicker label={say(lang(), "rules_building")} value={which()} onPick={setWhich} />
    </DocumentDraft>
  );
}
