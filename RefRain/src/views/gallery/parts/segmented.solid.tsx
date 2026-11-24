// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createSignal } from "solid-js";
import type { Choice, Group } from "../../parts/segmented";
import { WIRE_APIS } from "../../../core/commands";
import type { WireApi } from "../../../core/commands";
import { say } from "../../../core/lang";
import type { Chroma, Lighting, Motion } from "../../../core/appearance";
import { ui } from "../../../ui";
import Segmented from "../../parts/segmented.solid";
import { GalleryCase } from "../case";
import { storeValue } from "../../../core/solid_store";

const OPEN_AI: Group = { label: "OpenAI", tone: "plain" };
const ANTHROPIC: Group = { label: "Anthropic", tone: "alert" };

export default function SegmentedCases() {
  const lang = storeValue(ui().lang);
  const [chroma, setChroma] = createSignal<Chroma>("full");
  const [lighting, setLighting] = createSignal<Lighting>("system");
  const [raised, setRaised] = createSignal<Lighting>("dark");
  const [motion, setMotion] = createSignal<Motion>("off");
  const [wire, setWire] = createSignal<WireApi>("chat");
  const [gated, setGated] = createSignal<WireApi>("chat");
  const [unchosen, setUnchosen] = createSignal<WireApi | null>(null);
  function dialects(): readonly Choice<WireApi>[] {
    return WIRE_APIS.map((api) => ({
      value: api,
      label: api,
      group: api === "messages" ? ANTHROPIC : OPEN_AI,
    }));
  }
  function carried(): readonly Choice<WireApi>[] {
    return dialects().map((choice) =>
      choice.value === "responses"
        ? { ...choice, why: say(lang(), "setup_wire_api_unsupported") }
        : choice,
    );
  }
  return (
    <>
      <GalleryCase label="segmented · two cells">
        <Segmented
          label={say(lang(), "appearance_chroma")}
          options={[
            { value: "full", label: say(lang(), "appearance_chroma_full") },
            { value: "off", label: say(lang(), "appearance_chroma_none") },
          ]}
          held={chroma()}
          onPick={setChroma}
        />
      </GalleryCase>
      <GalleryCase label="segmented · three cells">
        <Segmented
          label={say(lang(), "appearance_lighting")}
          options={[
            { value: "system", label: say(lang(), "appearance_lighting_system") },
            { value: "dark", label: say(lang(), "appearance_lighting_dark") },
            { value: "light", label: say(lang(), "appearance_lighting_light") },
          ]}
          held={lighting()}
          onPick={setLighting}
        />
      </GalleryCase>
      <GalleryCase label="segmented · accent tone on a raised card">
        <div class="rounded-card bg-raised px-base py-snug">
          <Segmented
            label={say(lang(), "appearance_lighting")}
            tone="accent"
            options={[
              { value: "system", label: say(lang(), "appearance_lighting_system") },
              { value: "dark", label: say(lang(), "appearance_lighting_dark") },
              { value: "light", label: say(lang(), "appearance_lighting_light") },
            ]}
            held={raised()}
            onPick={setRaised}
          />
        </div>
      </GalleryCase>
      <GalleryCase label="segmented · three cells under two group headings">
        <Segmented
          label={say(lang(), "setup_wire_api")}
          options={dialects()}
          held={wire()}
          onPick={setWire}
        />
      </GalleryCase>
      <GalleryCase label="segmented · a cell that cannot be chosen">
        <Segmented
          label={say(lang(), "setup_wire_api")}
          options={carried()}
          held={gated()}
          onPick={setGated}
        />
      </GalleryCase>
      <GalleryCase label="segmented · nothing chosen yet">
        <Segmented
          label={say(lang(), "setup_wire_api")}
          options={dialects()}
          held={unchosen()}
          onPick={setUnchosen}
        />
      </GalleryCase>
      <GalleryCase label="segmented · motion off">
        <div data-motion="off">
          <Segmented
            label={say(lang(), "appearance_motion")}
            options={[
              { value: "system", label: say(lang(), "appearance_motion_system") },
              { value: "on", label: say(lang(), "appearance_motion_full") },
              { value: "off", label: say(lang(), "appearance_motion_off") },
            ]}
            held={motion()}
            onPick={setMotion}
          />
        </div>
      </GalleryCase>
    </>
  );
}
