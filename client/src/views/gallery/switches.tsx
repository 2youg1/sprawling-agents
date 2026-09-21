// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The sliding chooser in the six states it can be drawn in: two cells,
// three cells, three cells under two group headings, a cell that cannot
// be chosen and says why, a control nobody has answered yet, and the
// whole control with motion turned off.
//
// It is a file of its own rather than six more cases among the shared
// controls because each fixture holds the choice a person made in it,
// and six more signals is what would push that component past reading
// in one screen.

import { createSignal } from "solid-js";

import { WIRE_APIS, type WireApi } from "../../core/commands";
import type { Appearance } from "../../core/prefs";
import { useSay } from "../../ui";
import { Segmented, type Choice, type Group } from "../parts/segmented";
import { Case } from "./case";

// The two laboratories the three dialects come from, and the colour
// each is given. The sliding chooser is the only control in the client
// that paints by family, and the mapping is spelled at the call site
// because the control itself knows no vendor's name. The names are the
// laboratories' own and are the same letters in every language.
const OPEN_AI: Group = { label: "OpenAI", tone: "plain" };
const ANTHROPIC: Group = { label: "Anthropic", tone: "alert" };

export function Switches() {
  const say = useSay();
  const [chroma, setChroma] = createSignal<Appearance["chroma"]>("full");
  const [lighting, setLighting] = createSignal<Appearance["lighting"]>("system");
  const [motion, setMotion] = createSignal<Appearance["motion"]>("off");
  const [wire, setWire] = createSignal<WireApi>("chat");
  const [gated, setGated] = createSignal<WireApi>("chat");
  const [unchosen, setUnchosen] = createSignal<WireApi | null>(null);

  // The dialect list the provider form offers, grouped by the
  // laboratory whose wire it speaks. The words are the literal values
  // of Codex's `wire_api` key, which no language translates.
  const dialects = (): readonly Choice<WireApi>[] =>
    WIRE_APIS.map((api) => ({
      value: api,
      label: api,
      group: api === "messages" ? ANTHROPIC : OPEN_AI,
    }));

  // The same list as this city can serve it today: the middle dialect
  // is named, drawn and refused, so the reason arrives under the
  // pointer instead of after the click.
  const carried = (): readonly Choice<WireApi>[] =>
    dialects().map((choice) =>
      choice.value === "responses" ? { ...choice, why: say("setup_wire_api_unsupported") } : choice,
    );

  return (
    <>
      <Case label="segmented · two cells">
        <Segmented
          label={say("appearance_chroma")}
          options={[
            { value: "full", label: say("appearance_chroma_full") },
            { value: "off", label: say("appearance_chroma_none") },
          ]}
          held={chroma()}
          onPick={setChroma}
        />
      </Case>

      <Case label="segmented · three cells">
        <Segmented
          label={say("appearance_lighting")}
          options={[
            { value: "system", label: say("appearance_lighting_system") },
            { value: "dark", label: say("appearance_lighting_dark") },
            { value: "light", label: say("appearance_lighting_light") },
          ]}
          held={lighting()}
          onPick={setLighting}
        />
      </Case>

      <Case label="segmented · three cells under two group headings">
        <Segmented
          label={say("setup_wire_api")}
          options={dialects()}
          held={wire()}
          onPick={setWire}
        />
      </Case>

      <Case label="segmented · a cell that cannot be chosen">
        <Segmented label={say("setup_wire_api")} options={carried()} held={gated()} onPick={setGated} />
      </Case>

      {/* A caller holding nothing passes `null`, and the track answers
          by drawing no slider: no cell may report itself chosen when
          nobody has chosen. The first cell a person can choose is
          still the one tab stop, so the control is reachable by
          keyboard before it has an answer. */}
      <Case label="segmented · nothing chosen yet">
        <Segmented
          label={say("setup_wire_api")}
          options={dialects()}
          held={unchosen()}
          onPick={setUnchosen}
        />
      </Case>

      <Case label="segmented · motion off">
        <div data-motion="off">
          <Segmented
            label={say("appearance_motion")}
            options={[
              { value: "system", label: say("appearance_motion_system") },
              { value: "on", label: say("appearance_motion_full") },
              { value: "off", label: say("appearance_motion_off") },
            ]}
            held={motion()}
            onPick={setMotion}
          />
        </div>
      </Case>
    </>
  );
}
