// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, Show } from "solid-js";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import Tier from "../setup/tier_view";
import { SECTION } from "../talk/picker";
import { pickerOf, type PickerHands } from "../talk/picker_look";
import { inForce } from "../talk/picker_scene";
import SettingsRow, { ModelPicker } from "../talk/settings_row_view";
import { GalleryCase } from "./case";
import {
  AT_DEEPSEEK,
  AT_OLLAMA,
  AT_AGGREGATOR,
  MANY_MODELS,
  MANY_PROVIDERS,
  ONE_PROVIDER,
  pickerFacts,
} from "./picked";
const served = pickerFacts(MANY_PROVIDERS, AT_DEEPSEEK, null);
const stated = pickerFacts(MANY_PROVIDERS, AT_DEEPSEEK, "xhigh");
const local = pickerFacts(MANY_PROVIDERS, AT_OLLAMA, null);
const alone = pickerFacts(ONE_PROVIDER, AT_OLLAMA, null);
const aggregated = pickerFacts(MANY_MODELS, AT_AGGREGATOR, "medium");
const none = pickerFacts([], undefined, null);
const still = (): (() => void) => () => undefined;
const HANDS: PickerHands = {
  open: still,
  close: still,
  hold: still,
  keep: still,
  bound: still,
  cursor: still,
  focusFilter: still,
  holdToken: still,
  holdFilter: still,
  holdFrame: still,
};

export default function G2() {
  const lang = storeValue(ui().lang);
  const opened = createMemo(
    () =>
      pickerOf(
        lang(),
        aggregated,
        {
          open: true,
          segment: "model",
          pick: null,
          parent: null,
          pinned: { first: undefined, second: undefined },
          opened: inForce(aggregated),
          query: "",
          whole: [SECTION.second],
          kept: [],
          binding: null,
          active: null,
        },
        HANDS,
      )?.menu,
  );

  return (
    <>
      <GalleryCase label="settings row · before a session · picker and permissions">
        <SettingsRow picker={served} room={null} draws="everything" kept={false} />
      </GalleryCase>
      <GalleryCase label="settings row · no offered model · picker hidden">
        <SettingsRow picker={none} room={null} draws="everything" kept={false} />
      </GalleryCase>
      <GalleryCase label="settings row · the run policy's menu open">
        <div class="flex min-h-[14rem] flex-col justify-end">
          <SettingsRow picker={served} room={null} draws="everything" kept={false} menu="open" />
        </div>
      </GalleryCase>
      <GalleryCase label="picker · opened from the model · models first, providers of the chosen one">
        <div class="flex min-h-[46rem] flex-col justify-end">
          <SettingsRow picker={served} room={null} draws="everything" kept={false} menu="model" />
        </div>
      </GalleryCase>
      <GalleryCase label="picker · opened from the provider · a stated level this provider does not offer">
        <div class="flex min-h-[46rem] flex-col justify-end">
          <SettingsRow
            picker={stated}
            room={null}
            draws="everything"
            kept={false}
            menu="provider"
          />
        </div>
      </GalleryCase>
      <GalleryCase label="picker · opened from the level">
        <div class="flex min-h-[46rem] flex-col justify-end">
          <SettingsRow picker={served} room={null} draws="everything" kept={false} menu="level" />
        </div>
      </GalleryCase>
      <GalleryCase label="picker · a model with no thinking control · the band's room kept for the others">
        <div class="flex min-h-[46rem] flex-col justify-end">
          <SettingsRow picker={local} room={null} draws="everything" kept={false} menu="model" />
        </div>
      </GalleryCase>
      <GalleryCase label="picker · one provider, no thinking control anywhere · no band, no room for one">
        <div class="flex min-h-[30rem] flex-col justify-end">
          <SettingsRow picker={alone} room={null} draws="everything" kept={false} menu="model" />
        </div>
      </GalleryCase>
      <GalleryCase label="picker · more models than providers · providers first, five models and more">
        <div class="flex min-h-[50rem] flex-col justify-end">
          <SettingsRow
            picker={aggregated}
            room={null}
            draws="everything"
            kept={false}
            menu="model"
          />
        </div>
      </GalleryCase>
      <GalleryCase label="picker · the aggregator's models opened in full · scrolling in their room">
        <div class="flex min-h-[50rem] flex-col justify-end">
          <div class="relative h-touch">
            <Show when={opened()}>{(picked) => <ModelPicker menu={picked()} />}</Show>
          </div>
        </div>
      </GalleryCase>
      <GalleryCase label="settings row · after a session starts · no controls">
        <SettingsRow picker={served} room={null} draws="notice" kept={false} />
      </GalleryCase>
      <GalleryCase label="appearance · the tier card" width={390}>
        <Tier />
      </GalleryCase>
    </>
  );
}
