// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Address } from "../../wire";
import SettingsRow from "../talk/settings_row_view";
import { GalleryCase } from "./case";
import { AT_DEEPSEEK, MANY_PROVIDERS, pickerFacts } from "./picked";

const HERE = Address.make("hall/mayor");
const picker = pickerFacts(MANY_PROVIDERS, AT_DEEPSEEK, null);

export default function TalkRow() {
  return (
    <>
      <GalleryCase label="settings row · after a session starts · a sentence the link did not take">
        <SettingsRow picker={picker} room={HERE} draws="notice" kept />
      </GalleryCase>
      <GalleryCase label="settings row · before a session · a sentence the link did not take">
        <SettingsRow picker={picker} room={null} draws="everything" kept />
      </GalleryCase>
    </>
  );
}
