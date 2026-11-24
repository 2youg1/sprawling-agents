// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For } from "solid-js";
import { fill } from "../../core/lang";
import { recoveryFor } from "../../core/recovering";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import Banner from "../parts/banner.solid";
import Button from "../parts/button.solid";
import Field from "../parts/field.solid";
import Cheatsheet from "../parts/cheatsheet";
import Notice from "../parts/notice_view";
import MachineReport from "../machine/report.solid";
import MachineSkeleton from "../machine/skeleton.solid";
import MachineUnchecked from "../machine/unchecked.solid";
import KeysSection from "../setup/keys_view";
import ModelTable from "../setup/model_table.solid";
import AttachForm from "../setup/providers/form_view";
import EndpointList from "../setup/providers/endpoint_list.solid";
import Shelves from "../setup/shelves_view";
import EffortSection from "../shared/effort_view";
import Skyline from "../city/skyline.solid";
import { GalleryCase } from "./case";
import Stand from "./stand.solid";
import { ENDPOINTS, NOTICE_DAYS, PROBED, REFUSED_FIELD, TOASTS } from "./served";
import type { DoctorAnswer } from "../../wire";
import type { Lang } from "../../core/lang";
import { RELEASE_ALL } from "../../core/slash";
import { say } from "../../core/lang";
import type { Recovery } from "../../core/recovering";
import type { BuildingAnswer, CityAnswer } from "../../wire";
import { Address, NodeId, Tokens, UsdMicros } from "../../wire";
import Plan from "../building/plan.solid";

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

function RecoveryActions(props: { readonly choices: readonly Recovery[]; readonly lang: Lang }) {
  return (
    <For each={props.choices} keyed={(recovery) => wordOf(props.lang, recovery)}>
      {(recovery) => <Button label={wordOf(props.lang, recovery())} tone="quiet" />}
    </For>
  );
}

export default function Screens() {
  const lang = storeValue(ui().lang);
  const retryDetail = () =>
    [
      fill(say(lang(), "link_lost_detail"), { n: "3" }),
      fill(say(lang(), "link_unsent"), { n: "1" }),
    ].join(" · ");
  return (
    <>
      <GalleryCase label="machine · what this one has">
        <MachineReport answer={MACHINE} />
      </GalleryCase>
      <GalleryCase label="machine · being checked">
        <MachineSkeleton />
      </GalleryCase>
      <GalleryCase label="machine · not checked yet">
        <MachineUnchecked onRecheck={() => undefined} />
      </GalleryCase>
      <GalleryCase label="banner · the city is halted">
        <Banner
          text={say(lang(), "halt_title")}
          detail={fill(say(lang(), "halt_frozen"), { n: "3" })}
          weight="alert"
          action={<Button label={RELEASE_ALL} tone="secondary" />}
        />
      </GalleryCase>
      <GalleryCase label="banner · the link is lost">
        <Banner
          text={say(lang(), "link_lost")}
          detail={retryDetail()}
          weight="alert"
          action={
            <Button
              label={say(lang(), "link_retry_now")}
              tone="secondary"
              onPress={() => undefined}
            />
          }
        />
      </GalleryCase>
      <GalleryCase label="keys · the sheet every chord is read on">
        <Cheatsheet seat="specimen" onClose={() => undefined} />
      </GalleryCase>
      <GalleryCase label="keys · one row per action, rebound where it stands">
        <KeysSection />
      </GalleryCase>
      <GalleryCase label="provider · what is attached, keyed and unkeyed">
        <EndpointList answer={ENDPOINTS} />
      </GalleryCase>
      <GalleryCase label="provider · the form a key is filed through">
        <AttachForm />
      </GalleryCase>
      <GalleryCase label="models · the rows a probe answered">
        <ModelTable served={PROBED} about={null} onChosen={() => undefined} />
      </GalleryCase>
      <GalleryCase label="settings · how hard the city thinks">
        <EffortSection />
      </GalleryCase>
      <GalleryCase label="settings · where this city keeps its skills, one folder mounted from this computer">
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
              : undefined
          }
          children={() => <Shelves />}
        />
      </GalleryCase>
      <GalleryCase label="notices · the drawer, twelve entries across three days" width={440}>
        <div class="flex flex-col bg-chrome shadow-sheet">
          <div class="flex items-baseline justify-between gap-base px-base py-snug">
            <span class="text-label font-label">{say(lang(), "notices")}</span>
            <div class="flex items-center gap-tight">
              <Button label={say(lang(), "notices_mark_all")} tone="quiet" />
              <Button label={say(lang(), "notices_clear")} tone="quiet" />
            </div>
          </div>
          <div class="max-h-output overflow-y-auto">
            <For each={NOTICE_DAYS} keyed={(day) => day.heading}>
              {(day) => (
                <>
                  <div class="sticky top-0 bg-chrome px-base py-tight text-note text-text-faint">
                    {say(lang(), day().heading)}
                  </div>
                  <For each={day().rows} keyed={(entry) => entry.subject + entry.at}>
                    {(entry) => (
                      <Notice
                        seat="drawer"
                        weight="alert"
                        action={entry().action}
                        code={entry().code}
                        subject={entry().subject}
                        recovery={entry().recovery}
                        at={entry().at}
                        count={entry().count}
                        actions={() => (
                          <RecoveryActions choices={recoveryFor(entry())} lang={lang()} />
                        )}
                      />
                    )}
                  </For>
                </>
              )}
            </For>
          </div>
        </div>
      </GalleryCase>
      <GalleryCase label="notices · three toasts, stacked at the corner">
        <div class="flex min-h-output w-full flex-col items-end justify-end gap-snug rounded-card border border-dashed border-edge-input p-snug">
          <For each={TOASTS} keyed={(toast) => toast.subject + toast.at}>
            {(toast) => (
              <Notice
                seat="toast"
                weight="alert"
                action={toast().action}
                code={toast().code}
                subject={toast().subject}
                recovery={toast().recovery}
                at={toast().at}
                count={toast().count}
                actions={() => <RecoveryActions choices={recoveryFor(toast())} lang={lang()} />}
              />
            )}
          </For>
        </div>
      </GalleryCase>
      <GalleryCase label="notices · a stop key with no run in front">
        <div class="flex min-h-output w-full items-end justify-end rounded-card border border-dashed border-edge-input p-snug">
          <Notice seat="toast" weight="info" heading="no_run_in_front" next="stop_whole_city" />
        </div>
      </GalleryCase>
      <GalleryCase label="notices · inline under the field it belongs to">
        <div class="max-w-measure">
          <Field
            label={say(lang(), "setup_base_url")}
            value={REFUSED_FIELD.subject}
            help={say(lang(), "part_help_base_url")}
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
      </GalleryCase>
      <GalleryCase label="building · plan showing a share and a leaf count">
        <Plan answer={PLANNED} />
      </GalleryCase>
      <GalleryCase label="building · plan the city could not read">
        <Plan answer={UNPLANNED} />
      </GalleryCase>
      <GalleryCase label="city · a long building name stops at its own tower" width={1440}>
        <Skyline city={NEIGHBOURS} picked={null} onPick={() => undefined} />
      </GalleryCase>
    </>
  );
}
