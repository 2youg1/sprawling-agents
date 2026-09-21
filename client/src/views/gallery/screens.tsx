// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Whole screens, in the states the roadmap asks each of them for. Every
// one mounts the component the page mounts and hands it the props the
// page hands it; nothing here is a drawing of a screen.
//
// The sheet of keys covers the window it is opened over, so the box
// below it carries a transform, which is what makes a fixed box treat
// that box as its window.

import type { DoctorAnswer } from "../../wire";
import { useSay } from "../../ui";
import { MachineReport, MachineSkeleton, MachineUnchecked } from "../machine";
import { SkillsNote } from "../setup";
import { KeysSection } from "../setup/keys";
import { ModelTable } from "../setup/models";
import { AttachForm, EndpointList } from "../setup/providers";
import { EffortSection } from "../shared/effort";
import { Banner } from "../parts/banner";
import { Button } from "../parts/button";
import { Cheatsheet } from "../parts/kbd";
import { Case } from "./case";
import { ENDPOINTS, PROBED } from "./served";
import { Stand } from "./stand";

// One machine, with an item in each of the three states a person acts
// differently on: here, missing and required, missing and optional.
const MACHINE: DoctorAnswer = {
  items: [
    {
      name: "firefox",
      tier: "use",
      need: "required",
      enables: "the browser a city serves its pages to",
      state: { absent: { absence: "not_on_search_path" } },
      install: { command: { spelled: "winget install --id Mozilla.Firefox" } },
    },
    {
      name: "git",
      tier: "use",
      need: "required",
      enables: "the history every run is fenced against",
      state: {
        present: {
          at: "/usr/bin/git",
          version: { said: { text: "git version 2.55.0" } },
        },
      },
      install: { command: { spelled: "winget install --id Git.Git" } },
    },
    {
      name: "chromedriver",
      tier: "use",
      need: "optional",
      enables: "the browser tool against Chromium",
      state: { absent: { absence: "not_on_search_path" } },
      install: "unknown_platform",
    },
  ],
  tiers: [
    { tier: "use", missing: ["firefox"] },
    { tier: "develop", missing: [] },
  ],
};

export function Screens() {
  const say = useSay();
  return (
    <>
      <Case label="machine · what this one has">
        <MachineReport answer={MACHINE} />
      </Case>

      <Case label="machine · being checked">
        <MachineSkeleton />
      </Case>

      <Case label="machine · not checked yet">
        <MachineUnchecked onRecheck={() => undefined} />
      </Case>

      <Case label="banner · the city is halted">
        <Banner
          text={say("halt_title")}
          detail={say("halt_frozen", { n: "3" })}
          weight="alert"
          action={<Button label={say("city_release")} tone="secondary" />}
        />
      </Case>

      <Case label="keys · the sheet every chord is read on">
        <div class="relative h-screen transform-gpu overflow-hidden">
          <Cheatsheet onClose={() => undefined} />
        </div>
      </Case>

      <Case label="keys · one row per action, rebound where it stands">
        <KeysSection />
      </Case>

      <Case label="provider · what is attached, keyed and unkeyed">
        <EndpointList answer={ENDPOINTS} />
      </Case>

      <Case label="provider · the form a key is filed through">
        <AttachForm />
      </Case>

      <Case label="models · the rows a probe answered">
        <ModelTable served={PROBED} onChosen={() => undefined} />
      </Case>

      {/* Nobody having chosen is a state of its own and the state a new
          city is in: the cell leads the track, and the line under it
          says the provider decides. A level chosen is the other
          reading, and the two differ in exactly one sentence. */}
      <Case label="settings · how hard the city thinks, nobody having said">
        <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} effort={null}>
          <EffortSection />
        </Stand>
      </Case>

      <Case label="settings · how hard the city thinks, a level chosen">
        <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} effort="high">
          <EffortSection />
        </Stand>
      </Case>

      <Case label="settings · where this city keeps its skills">
        <SkillsNote />
      </Case>
    </>
  );
}
