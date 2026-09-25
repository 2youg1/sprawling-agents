<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The settings page, whole, in the states a person has room for.
  //
  // **The page is the one screen whose defects were only visible at a
  // width it was never measured at.** Three columns inside a container
  // centred and capped at the page width put the model table's id
  // column under forty points and wrapped an English sentence over a
  // dozen lines, and `cargo xtask render` had never opened `#/setup`
  // at all. A fixture that mounts the page itself is what makes the
  // layout a thing the gate holds to, rather than a thing somebody
  // has to open a city to see.
  //
  // **The page is mounted, not drawn.** Every case below hands `Setup`
  // the answers a city would hand it - an endpoint list, a machine
  // survey, nothing at all - so the two-column layout, the group
  // headings, the collapsed `config.toml` and the card grids are the
  // shipped ones. The rank is `section` because the gallery already
  // has the page's first heading, and a document has exactly one.
  //
  // The full-page cases state the two windows the roadmap measures the
  // settled screen at: 1280, where the accounts form and the proof
  // beside it sit side by side, and 820, where the group navigation
  // folds over and the columns become one. Both mount the real page
  // with fixture answers - the endpoints above, the survey below, and
  // the shelves `shelved.svelte` stands up - so what the width rule
  // does to each content kind is what a person meets.
  //
  // The machine below is this section's own: three required programs,
  // one of them here and two missing, and one optional driver with no
  // recipe for this platform. It is a different survey from the one
  // `screens.svelte` draws, whose subject is the sandbox arm and the
  // tier verdicts; this one's subject is the width of a card at the
  // front of a grid.

  import type { DoctorAnswer } from "../../wire";

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
        name: "firefox",
        tier: "use",
        need: "required",
        enables: "the browser a city serves its pages to",
        state: { absent: { absence: "not_on_search_path" } },
        install: { command: { spelled: "winget install --id Mozilla.Firefox" } },
      },
      {
        name: "ripgrep",
        tier: "use",
        need: "required",
        enables: "the search a resident runs before it writes anything",
        state: { absent: { absence: "not_on_search_path" } },
        install: { manual: { how: "download the release archive and put rg on PATH" } },
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
      { tier: "use", missing: ["firefox", "ripgrep"] },
      { tier: "develop", missing: [] },
    ],
  };
</script>

<script lang="ts">
  import { MachineReport, MachineUnchecked } from "../machine.svelte";
  import Setup from "../setup.svelte";
  import { ENDPOINTS } from "./served";
  import Case from "./case.svelte";
  import { answered } from "./shelved.svelte";
  import Stand from "./stand.svelte";
</script>

<!-- What the dependency group's body is, per case. The real one asks
the city; these two are the survey above and the state before anyone
has asked. -->
{#snippet cards()}
  <MachineReport answer={MACHINE} onInstall={() => undefined} />
{/snippet}

{#snippet unchecked()}
  <MachineUnchecked onRecheck={() => undefined} />
{/snippet}

<Case label="settings · accounts, and the models the city may call" width={1440}>
  <Setup
    rank="section"
    group="accounts"
    onPick={() => undefined}
    endpoints={ENDPOINTS}
    autonomy={undefined}
    onAutonomy={() => undefined}
    dependency={unchecked}
  />
</Case>

<!-- The mcp reading needs no case of its own any more: its door is a
link in every group's navigation now, so each case above and below
draws it where a person meets it. -->

<Case label="settings · dependencies, one card per program" width={1440}>
  <Setup
    rank="section"
    group="tools"
    onPick={() => undefined}
    endpoints={undefined}
    autonomy={undefined}
    onAutonomy={() => undefined}
    dependency={cards}
  />
</Case>

<Case label="settings · dependencies, nobody has looked yet" width={1440}>
  <Setup
    rank="section"
    group="tools"
    onPick={() => undefined}
    endpoints={undefined}
    autonomy={undefined}
    onAutonomy={() => undefined}
    dependency={unchecked}
  />
</Case>

<Case label="settings · about, one row and the check that is never automatic" width={1440}>
  <Setup
    rank="section"
    group="about"
    onPick={() => undefined}
    endpoints={undefined}
    autonomy={undefined}
    onAutonomy={() => undefined}
    dependency={unchecked}
  />
</Case>

<!-- The three content kinds at the two windows the roadmap settles
the screen at. Each case states the window it is a page of: 1280 is
the wide reading, where the provider form keeps its 760 and the proof
drops beside it; 820 is the narrow one, where the group navigation
turns into a row across the top and every grid is down to one column. -->
<Case label="setup · accounts at 1280" width={1280}>
  <Setup
    rank="section"
    group="accounts"
    onPick={() => undefined}
    endpoints={ENDPOINTS}
    autonomy={undefined}
    onAutonomy={() => undefined}
    dependency={unchecked}
  />
</Case>

<Case label="setup · accounts at 820" width={820}>
  <Setup
    rank="section"
    group="accounts"
    onPick={() => undefined}
    endpoints={ENDPOINTS}
    autonomy={undefined}
    onAutonomy={() => undefined}
    dependency={unchecked}
  />
</Case>

<Case label="setup · tools at 1280" width={1280}>
  <Setup
    rank="section"
    group="tools"
    onPick={() => undefined}
    endpoints={undefined}
    autonomy={undefined}
    onAutonomy={() => undefined}
    dependency={cards}
  />
</Case>

<Case label="setup · tools at 820" width={820}>
  <Setup
    rank="section"
    group="tools"
    onPick={() => undefined}
    endpoints={undefined}
    autonomy={undefined}
    onAutonomy={() => undefined}
    dependency={cards}
  />
</Case>

<Case label="setup · skills at 1280" width={1280}>
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answered}>
    <Setup
      rank="section"
      group="skills"
      onPick={() => undefined}
      endpoints={undefined}
      autonomy={undefined}
      onAutonomy={() => undefined}
      dependency={unchecked}
    />
  </Stand>
</Case>

<Case label="setup · skills at 820" width={820}>
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answered}>
    <Setup
      rank="section"
      group="skills"
      onPick={() => undefined}
      endpoints={undefined}
      autonomy={undefined}
      onAutonomy={() => undefined}
      dependency={unchecked}
    />
  </Stand>
</Case>
