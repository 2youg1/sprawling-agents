<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // Whole screens, in the states the roadmap asks each of them for.
  // Every one mounts the component the page mounts and hands it the
  // props the page hands it; nothing here is a drawing of a screen.
  //
  // The sheet of keys covers the window it is opened over, so the box
  // below it carries a transform, which is what makes a fixed box
  // treat that box as its window.
  //
  // **The three notice seats live here because one refusal has three
  // homes on screen and no seat of its own file** (client-SPEC 4-35):
  // the drawer that keeps a day of them, the corner a toast rises
  // into, and the line under a field that a person is still typing in.
  // Each draws the same `parts/notice.svelte`, so the three cases read
  // as three placements of one thing rather than three notice
  // drawings.

  import type { DoctorAnswer } from "../../wire";
  import type { Lang } from "../../core/lang";
  import { say } from "../../core/lang";
  import type { Recovery } from "../../core/recovering";
  import type { BuildingAnswer } from "../../wire";
  import { Address, NodeId, Tokens, UsdMicros } from "../../wire";
  import Plan from "../building/plan.svelte";

  // One plan in the state the two figures exist for: a branch that was
  // divided generously, so the share says more than the count does.
  // Three leaves, one finished, one running, one held up - and 50%
  // behind the reader while only a third of the leaves are done, which
  // is the difference the second figure is drawn to let somebody see.
  const PLANNED: BuildingAnswer = {
    addr: Address.make("lab"),
    archive: [],
    blocked: [
      { line: "waiting on the endpoint to answer", source: NodeId.make("3"), waiting: 2 },
    ],
    docs: [],
    mcp: [],
    plan: [
      {
        evidence: null,
        item: "read the shape before changing it",
        leaf: true,
        needs: [],
        node: NodeId.make("1"),
        ready: true,
        share_ppb: 500_000_000,
        status: "done",
      },
      {
        evidence: null,
        item: "wire the two halves together",
        leaf: true,
        needs: [NodeId.make("1")],
        node: NodeId.make("2"),
        ready: true,
        share_ppb: 300_000_000,
        status: "in_progress",
      },
      {
        evidence: null,
        item: "cut a release",
        leaf: true,
        needs: [NodeId.make("2")],
        node: NodeId.make("3"),
        ready: false,
        share_ppb: 200_000_000,
        status: "blocked",
      },
    ],
    problems: [],
    progress: {
      planned: { blocked: 1, blocked_ppb: 200_000_000, done: 1, done_ppb: 500_000_000, total: 3 },
    },
    rooms: [],
    sandbox: null,
  };

  // The other face. A plan the city could not read has no denominator,
  // and the interface says so rather than painting a fraction it does
  // not have (kernel::completion, A17's type half).
  const UNPLANNED: BuildingAnswer = {
    ...PLANNED,
    plan: [],
    problems: ["row 2 has 4 columns; the table has six"],
    progress: {
      unplanned: {
        budget: { tokens: Tokens.make(0), usd: UsdMicros.make(0) },
        steps: 7,
      },
    },
  };

  // One machine, with an item in each of the three states a person
  // acts differently on: here, missing and required, missing and
  // optional.
  //
  // The confinement it reports is the Windows arm on purpose. It is
  // the one that keeps some axes and not others - a job object ends a
  // process tree and caps what it may spend, and does nothing at all
  // about the network - so it is the arm that proves the page draws a
  // guarantee list rather than a yes or a no. "Windows has no
  // sandbox" would have been easier to draw and would have been false;
  // a machine that says it is boxed in while the box has no lid is
  // worse than one that says it has no box.
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

  // The label of one recovery action. A command is spelled as itself
  // in both languages; a link action has no spelling and takes the
  // word its own row carries (client-SPEC 4-10, `core/recovering.ts`).
  function wordOf(lang: Lang, recovery: Recovery): string {
    switch (recovery.kind) {
      case "command":
        return recovery.spelled;
      case "reconnect":
      case "settings":
        return say(lang, recovery.verb);
    }
  }
</script>

<script lang="ts">
  import { fill } from "../../core/lang";
  import { recoveryFor } from "../../core/recovering";
  import { ui } from "../../ui";
  import Banner from "../parts/banner.svelte";
  import Button from "../parts/button.svelte";
  import Field from "../parts/field.svelte";
  import Cheatsheet from "../parts/kbd.svelte";
  import Notice from "../parts/notice.svelte";
  import { MachineReport, MachineSkeleton, MachineUnchecked } from "../machine.svelte";
  import { KeysSection } from "../setup/keys";
  import { ModelTable } from "../setup/models";
  import { AttachForm, EndpointList } from "../setup/providers";
  import { Shelves } from "../setup/skills";
  import { EffortSection } from "../shared/effort";
  import Case from "./case.svelte";
  import { ENDPOINTS, NOTICE_DAYS, PROBED, REFUSED_FIELD, TOASTS } from "./served";

  const { lang } = ui();
</script>

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
    text={say($lang, "halt_title")}
    detail={fill(say($lang, "halt_frozen"), { n: "3" })}
    weight="alert"
  >
    {#snippet action()}
      <Button label={say($lang, "city_release")} tone="secondary" />
    {/snippet}
  </Banner>
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

<!-- One state, because this section holds no answer: where the level
is decided is the same sentence whatever the city has been told, and
the choosing happens over the composer. -->
<Case label="settings · how hard the city thinks">
  <EffortSection />
</Case>

<Case label="settings · where this city keeps its skills">
  <Shelves />
</Case>

<!-- The notice drawer: a day of refusals kept where a person can come
back to them, grouped by day under a sticky heading, each entry
carrying the count of the refusals that arrived the same way and the
actions `core/recovering.ts` maps from its code. The waiting questions
sit above these in the shipped drawer and keep their own fixture. -->
<Case label="notices · the drawer, twelve entries across three days" width={440}>
  <div class="flex flex-col bg-chrome shadow-sheet">
    <div class="flex items-baseline justify-between gap-base px-base py-snug">
      <span class="text-label font-label">{say($lang, "notices")}</span>
      <div class="flex items-center gap-tight">
        <Button label={say($lang, "notices_mark_all")} tone="quiet" />
        <Button label={say($lang, "notices_clear")} tone="quiet" />
      </div>
    </div>
    <div class="max-h-output overflow-y-auto">
      {#each NOTICE_DAYS as day (day.heading)}
        <div class="sticky top-0 bg-chrome px-base py-tight text-note text-text-faint">
          {say($lang, day.heading)}
        </div>
        {#each day.rows as entry (entry.subject + entry.at)}
          <Notice
            seat="drawer"
            weight="alert"
            action={entry.action}
            code={entry.code}
            subject={entry.subject}
            recovery={entry.recovery}
            at={entry.at}
            count={entry.count}
          >
            {#snippet actions()}
              {#each recoveryFor(entry.code) as recovery (recovery.kind === "command" ? recovery.spelled : recovery.verb)}
                <Button label={wordOf($lang, recovery)} tone="quiet" />
              {/each}
            {/snippet}
          </Notice>
        {/each}
      {/each}
    </div>
  </div>
</Case>

<!-- Three toasts in the corner they land in. The stack is capped at
three by whoever keeps the list: a fourth refusal goes to the drawer
rather than into a taller corner. -->
<Case label="notices · three toasts, stacked at the corner">
  <div class="flex min-h-output w-full flex-col items-end justify-end gap-snug rounded-card border border-dashed border-edge-input p-snug">
    {#each TOASTS as toast (toast.subject + toast.at)}
      <Notice
        seat="toast"
        weight="alert"
        action={toast.action}
        code={toast.code}
        subject={toast.subject}
        recovery={toast.recovery}
        at={toast.at}
        count={toast.count}
      >
        {#snippet actions()}
          {#each recoveryFor(toast.code) as recovery (recovery.kind === "command" ? recovery.spelled : recovery.verb)}
            <Button label={wordOf($lang, recovery)} tone="quiet" />
          {/each}
        {/snippet}
      </Notice>
    {/each}
  </div>
</Case>

<!-- One refusal with a home: it sits under the field it was refused
at, and the field carries no error of its own - the notice is the
error, and both red markings at once would say it twice. -->
<Case label="notices · inline under the field it belongs to">
  <div class="max-w-measure">
    <Field
      label={say($lang, "setup_base_url")}
      value={REFUSED_FIELD.subject}
      help={say($lang, "part_help_base_url")}
      onInput={() => undefined}
    />
    <Notice
      seat="inline"
      weight="alert"
      action={REFUSED_FIELD.action}
      code={REFUSED_FIELD.code}
      subject={REFUSED_FIELD.subject}
      recovery={REFUSED_FIELD.recovery}
      at={REFUSED_FIELD.at}
      count={REFUSED_FIELD.count}
    />
  </div>
</Case>

<!-- The two figures the plan carries, and the face that has neither.
     One case each, because a renderer is told to handle both and a
     fixture that only ever drew the happy face would leave the other
     unpainted and unjudged. -->
<Case label="building · plan showing a share and a leaf count">
  <Plan answer={PLANNED} />
</Case>

<Case label="building · plan the city could not read">
  <Plan answer={UNPLANNED} />
</Case>
