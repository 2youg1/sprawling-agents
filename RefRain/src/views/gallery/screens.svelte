<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import type { DoctorAnswer } from "../../wire";
  import type { Lang } from "../../core/lang";
  import { RELEASE_ALL } from "../../core/slash";
  import { say } from "../../core/lang";
  import type { Recovery } from "../../core/recovering";
  import type { BuildingAnswer, CityAnswer } from "../../wire";
  import { Address, NodeId, Tokens, UsdMicros } from "../../wire";
  import Plan from "../building/plan.svelte";

  const PLANNED: BuildingAnswer = {
    addr: Address.make("lab"),
    archive: [],
    blocked: [{ line: "waiting on the endpoint to answer", source: NodeId.make("3"), waiting: 2 }],
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

  const idle = {
    unplanned: { budget: { tokens: Tokens.make(0), usd: UsdMicros.make(0) }, steps: 0 },
  };
  const NEIGHBOURS: CityAnswer = {
    active: 0,
    frozen: 0,
    halted: [],
    pursuits: [],
    runs: [],
    buildings: [
      {
        addr: Address.make("notes"),
        adoptable: false,
        blocked: [],
        problems: [],
        progress: idle,
        ready: 0,
      },
      {
        addr: Address.make("the model changed from one provider to the next"),
        adoptable: false,
        blocked: [],
        problems: [],
        progress: idle,
        ready: 0,
      },
    ],
  };

  const MACHINE: DoctorAnswer = {
    core: "raised",
    scanning: "does_not_apply",
    custody: { keeps: "across_reboots", store: "platform_service" },
    sandbox: {
      arm: "windows_job_object",
      named: "native",
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
        state: { absent: { absence: "not_on_search_path" } },
        install: { command: { spelled: "winget install --id Mozilla.Firefox" } },
      },
      {
        name: "git",
        tier: "use",
        need: "required",
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
        state: { absent: { absence: "not_on_search_path" } },
        install: "unknown_platform",
      },
    ],
    tiers: [
      { tier: "use", missing: ["firefox"] },
      { tier: "develop", missing: [] },
    ],
  };

  function wordOf(lang: Lang, recovery: Recovery): string {
    switch (recovery.kind) {
      case "command":
        return recovery.spelled;
      case "reconnect":
      case "settings":
      case "reload":
        return say(lang, recovery.verb);
      case "form":
        return say(lang, recovery.label);
    }
  }
</script>

<script lang="ts">
  import { fill } from "../../core/lang";
  import { recoveryFor } from "../../core/recovering";
  import { ui } from "../../ui";
  import Banner from "../parts/banner.svelte";
  import LinkBanner from "../link_banner.svelte";
  import Button from "../parts/button.svelte";
  import Field from "../parts/field.svelte";
  import Cheatsheet from "../parts/kbd.svelte";
  import Notice from "../parts/notice.svelte";
  import { MachineReport, MachineSkeleton, MachineUnchecked } from "../machine.svelte";
  import { KeysSection } from "../setup/keys";
  import { ModelTable } from "../setup/models";
  import { AttachForm, EndpointList } from "../setup/providers";
  import { Shelves } from "../setup/skills";
  import Stand from "./stand.svelte";
  import { EffortSection } from "../shared/effort";
  import Skyline from "../city/skyline.svelte";
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
      <Button label={RELEASE_ALL} tone="secondary" />
    {/snippet}
  </Banner>
</Case>

<Case label="banner · the link is lost">
  <LinkBanner attempt={3} unsent={1} onRetry={() => undefined} />
</Case>

<Case label="keys · the sheet every chord is read on">
  <Cheatsheet seat="specimen" onClose={() => undefined} />
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
  <ModelTable served={PROBED} about={null} onChosen={() => undefined} />
</Case>

<Case label="settings · how hard the city thinks">
  <EffortSection />
</Case>

<Case label="settings · where this city keeps its skills, one folder mounted from this computer">
  <Stand
    link={{ kind: "live", city: "sprawling" }}
    unread={[]}
    waiting={[]}
    answers={(query) =>
      typeof query === "object" && "skills" in query
        ? {
            skills: {
              building: query.skills.building,
              external_shelves: ["~/.claude/skills"],
              missing: [],
              skills: [],
            },
          }
        : undefined}
  >
    <Shelves />
  </Stand>
</Case>

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
              {#each recoveryFor(entry) as recovery (wordOf($lang, recovery))}
                <Button label={wordOf($lang, recovery)} tone="quiet" />
              {/each}
            {/snippet}
          </Notice>
        {/each}
      {/each}
    </div>
  </div>
</Case>

<Case label="notices · three toasts, stacked at the corner">
  <div
    class="flex min-h-output w-full flex-col items-end justify-end gap-snug rounded-card border border-dashed border-edge-input p-snug"
  >
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
          {#each recoveryFor(toast) as recovery (wordOf($lang, recovery))}
            <Button label={wordOf($lang, recovery)} tone="quiet" />
          {/each}
        {/snippet}
      </Notice>
    {/each}
  </div>
</Case>

<Case label="notices · a stop key with no run in front">
  <div
    class="flex min-h-output w-full items-end justify-end rounded-card border border-dashed border-edge-input p-snug"
  >
    <Notice seat="toast" weight="info" heading="no_run_in_front" next="stop_whole_city" />
  </div>
</Case>

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

<Case label="building · plan showing a share and a leaf count">
  <Plan answer={PLANNED} />
</Case>

<Case label="building · plan the city could not read">
  <Plan answer={UNPLANNED} />
</Case>

<Case label="city · a long building name stops at its own tower" width={1440}>
  <Skyline city={NEIGHBOURS} picked={null} onPick={() => undefined} />
</Case>
