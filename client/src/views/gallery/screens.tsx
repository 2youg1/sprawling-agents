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
import { Shelves } from "../setup/skills";
import { KeysSection } from "../setup/keys";
import { ModelTable } from "../setup/models";
import { AttachForm, EndpointList } from "../setup/providers";
import { EffortSection } from "../shared/effort";
import { Banner } from "../parts/banner";
import { Button } from "../parts/button";
import { Cheatsheet } from "../parts/kbd";
import { Case } from "./case";
import { ENDPOINTS, PROBED } from "./served";

// One machine, with an item in each of the three states a person acts
// differently on: here, missing and required, missing and optional.
//
// The confinement it reports is the Windows arm on purpose. It is the
// one that keeps some axes and not others - a job object ends a
// process tree and caps what it may spend, and does nothing at all
// about the network - so it is the arm that proves the page draws a
// guarantee list rather than a yes or a no. "Windows has no sandbox"
// would have been easier to draw and would have been false; a machine
// that says it is boxed in while the box has no lid is worse than one
// that says it has no box.
const MACHINE: DoctorAnswer = {
  custody: { keeps: "across_reboots", store: "platform_service" },
  sandbox: {
    arm: "windows_job_object",
    coverage: [
      { axis: "filesystem", kept: "kept" },
      { axis: "process_tree", kept: "kept" },
      { axis: "resources", kept: "kept" },
      { axis: "network", kept: "not_kept" },
      { axis: "user", kept: "not_kept" },
    ],
  },
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

      {/* One state, because this section holds no answer: where the
          level is decided is the same sentence whatever the city has
          been told, and the choosing happens over the composer. */}
      <Case label="settings · how hard the city thinks">
        <EffortSection />
      </Case>

      <Case label="settings · where this city keeps its skills">
        <Shelves />
      </Case>
    </>
  );
}
