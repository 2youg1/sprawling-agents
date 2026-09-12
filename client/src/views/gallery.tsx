// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Every state worth looking at, on fixtures, with no city behind it.
//
// It is a route rather than a build flag for two reasons: the gate that
// measures it opens the same bundle a person runs, and a person deciding
// between two ways a thing could look can open it on their own machine
// without standing up a provider first.

import { For, createSignal, type JSX } from "solid-js";

import { sendingInto, type Doing, type Sending } from "../core/belief";
import { EFFORTS } from "../core/prefs";
import type { ModelFact } from "../core/probed";
import { offered } from "../core/slash";
import type { ApprovalClass, ApprovalItem, DoctorAnswer, EndpointsAnswer } from "../wire";
import { ApprovalId, Locator, TimeMs } from "../wire";
import { useSay } from "../ui";
import { MachineReport, MachineSkeleton, MachineUnchecked } from "./machine";
import { SkillsNote } from "./setup";
import { KeysSection } from "./setup/keys";
import { EffortChoice, ModelTable } from "./setup/models";
import { AttachForm, EndpointList } from "./setup/providers";
import { Cheatsheet } from "./parts/kbd";
import { Popover } from "./parts/popover";
import { Badge } from "./parts/badge";
import { Banner } from "./parts/banner";
import { Button } from "./parts/button";
import { Combobox } from "./parts/combobox";
import { Dialog } from "./parts/dialog";
import { EmptyState } from "./parts/empty";
import { Field } from "./parts/field";
import { Notice } from "./parts/notice";
import { Progress } from "./parts/progress";
import { Row } from "./parts/row";
import { Skeleton } from "./parts/skeleton";
import { Table, type Column } from "./parts/table";
import { Tabs } from "./parts/tabs";
import { Composer } from "./talk/composer";
import { WaitingCards } from "./talk/waiting";

// One row of the model table a provider's probe answers with, in the
// shape §3.4 asks for: an id, what it can read, and what it may write -
// the last of which a person corrects when the probe could not read it.
interface ModelRow {
  readonly id: string;
  readonly context: string;
  readonly ceiling: string;
}

const MODELS: readonly ModelRow[] = [
  { id: "meta/muse-spark-1.3-contributor", context: "131072", ceiling: "8192" },
  { id: "anthropic/claude-fable-5.1", context: "204800", ceiling: "64000" },
  { id: "openai/gpt-nucleus-6", context: "400000", ceiling: "" },
];

// What one provider's probe answered, as a person meets it: three text
// models across three vendors, and one the text-only switch hides.
// Two rows state their own ceilings and prices, one states nothing but a
// name, and one is marked as video by the provider rather than by its
// spelling - which is the whole range this table has to render.
const PROBED: readonly ModelFact[] = [
  {
    id: "anthropic/claude-fable-5.1",
    contextTokens: 204_800,
    maxOutputTokens: 64_000,
    inputModalities: ["image", "text"],
    inputPrice: "0.000003",
    outputPrice: "0.000015",
  },
  {
    id: "openai/gpt-nucleus-6",
    contextTokens: 400_000,
    maxOutputTokens: null,
    inputModalities: ["text"],
    inputPrice: null,
    outputPrice: null,
  },
  {
    id: "openai/sora-2",
    contextTokens: null,
    maxOutputTokens: null,
    inputModalities: ["video"],
    inputPrice: null,
    outputPrice: null,
  },
  {
    id: "meta/muse-spark-1.3-contributor",
    contextTokens: null,
    maxOutputTokens: null,
    inputModalities: [],
    inputPrice: null,
    outputPrice: null,
  },
];

// Two attached providers, one with a key filed for it and one without,
// which is the difference the endpoint list exists to show.
const ENDPOINTS: EndpointsAnswer = {
  chosen: [{ endpoint: "zenmux", model: "anthropic/claude-fable-5.1", tag: "main" }],
  endpoints: [
    {
      base_url: "https://api.zenmux.ai/v1",
      dialect: "open_ai",
      has_credential: true,
      label: "ZenMux",
      local: false,
      models: ["anthropic/claude-fable-5.1", "openai/gpt-nucleus-6"],
      name: "zenmux",
    },
    {
      base_url: "http://127.0.0.1:11434/v1",
      dialect: "open_ai",
      has_credential: false,
      label: "local",
      local: true,
      models: ["local/qwen3"],
      name: "local",
    },
  ],
};

// The four readings of one run, which is what the lens switcher is for.
const LENSES = ["run_turns", "run_context", "run_changes", "run_evidence"] as const;

// Three things a person can be asked, in the three shapes the cards
// take: one on its own, several identical ones answered together, and
// one raised by a run that began with somebody else's words - which is
// never grouped with anything.
const waiting = (
  id: string,
  actor: string,
  what: string,
  key: readonly [ApprovalClass, string],
  at: number,
  tainted: boolean,
): ApprovalItem => ({
  id: ApprovalId.make(id),
  actor,
  action_desc: what,
  artifact: Locator.make("cas:b3-0000000000000000000000000000000000000000000000000000000000000000"),
  cluster_key: { class: key[0], detail: key[1] },
  created: TimeMs.make(at),
  source: "gate",
  tainted,
});

// Three things a person can be asked, in the three shapes the cards
// take: one on its own, several identical ones answered together, and
// one raised by a run that began with somebody else's words - which is
// never grouped with anything.
const WAITING: readonly ApprovalItem[] = [
  waiting("ai_1", "lab/east", "exec: rm -rf target", ["undoable", "rm"], 1, false),
  waiting("ai_2", "lab/west", "edit: the building's own rules", ["governance", "rules"], 2, false),
  waiting("ai_3", "lab/west", "edit: the building's own rules", ["governance", "rules"], 3, false),
  waiting("ai_4", "hall/mayor", "browser: open a page somebody linked", ["agent_question", "open"], 4, true),
];

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

// The postures a run can be in, in the order a dispatch meets them.
const POSTURES: readonly Doing[] = [
  { kind: "thinking" },
  { kind: "calling", tool: "exec", subject: "just check" },
  { kind: "waiting" },
  { kind: "frozen", completion: "done" },
];

// How many characters of a live turn are drawn faint. Kept in step with
// `thread.tsx` by being shown here at several lengths rather than by a
// second copy of the number: what this page is for is deciding whether
// the number is right.
const SAYING = [
  "on",
  "on it — reading the city",
  "on it — reading the city, then writing the plan it asks for",
];

function Case(props: { readonly label: string; readonly children: JSX.Element }) {
  return (
    <section class="mb-wide" aria-label={props.label}>
      <div class="mb-tight text-note text-text-disabled">{props.label}</div>
      {props.children}
    </section>
  );
}

export function Gallery() {
  const say = useSay();
  return (
    <div class="mx-auto w-full max-w-talk px-pane py-pane">
      <h1 class="mb-wide text-heading text-text">{say("gallery_title")}</h1>

      <For each={POSTURES}>
        {(doing) => (
          <Case label={`${doing.kind} · ${sendingInto(doing)}`}>
            <Composer
              placeholder={say("talk_placeholder_mayor")}
              sending={sendingInto(doing) satisfies Sending}
              hearing={doing.kind === "thinking"}
              onSend={() => false}
              onStop={() => false}
            />
          </Case>
        )}
      </For>

      <For each={SAYING}>
        {(text) => (
          <Case label={`saying · ${String(text.length)}`}>
            <div class="whitespace-pre-wrap leading-relaxed text-body">
              {text.slice(0, -10)}
              <span class="text-text-faint">{text.slice(-10)}</span>
              <span class="ml-tight inline-block h-caret w-hair animate-pulse bg-accent align-text-bottom" />
            </div>
          </Case>
        )}
      </For>

      <Case label="machine">
        <MachineReport answer={MACHINE} />
      </Case>

      <Case label="waiting">
        <WaitingCards items={WAITING} />
      </Case>

      <Screens />
      <Parts />
    </div>
  );
}

// Whole screens, in the states §13.1 of the roadmap asks each of them
// for. Every one mounts the component the page mounts and hands it the
// props the page hands it; nothing here is a drawing of a screen.
//
// Two of them need room the page gives them and a fixture does not. The
// list that opens over the composer opens *upward*, so the padding above
// it is the room the foot of a page has; the sheet `?` opens covers the
// window it is opened over, so the box below carries a transform, which
// is what makes a fixed box treat that box as its window.
function Screens() {
  const say = useSay();
  return (
    <>
      <Case label="composer · an empty room opens in the middle">
        <div class="flex flex-col items-center gap-base py-section text-center">
          <p class="text-heading font-heading text-text-disabled">{say("talk_empty_mayor")}</p>
          <p class="text-note text-text-faint">{say("talk_opening_mayor")}</p>
          <div class="w-full">
            <Composer
              placeholder={say("talk_placeholder_mayor")}
              sending="dispatch"
              onSend={() => false}
              onStop={() => false}
            />
          </div>
        </div>
      </Case>

      <Case label="composer · docked once the room has a thread">
        <div class="px-pane pb-pane">
          <Composer
            placeholder={say("talk_placeholder_mayor")}
            sending="dispatch"
            hearing
            onSend={() => false}
            onStop={() => false}
          />
        </div>
      </Case>

      <Case label="menu · a line that begins with a slash">
        <div class="pt-palette">
          <div class="pt-output">
            <div class="relative">
              <Popover
                label={say("talk_commands")}
                columns={[
                  {
                    id: "commands",
                    label: say("talk_commands"),
                    items: offered("/").map((each) => ({
                      id: each.spelling,
                      label: each.spelling,
                      hint: each.grammar === "" ? say(each.about) : `${each.grammar} · ${say(each.about)}`,
                    })),
                  },
                ]}
                onApply={() => undefined}
                onClose={() => undefined}
                bind={() => undefined}
              />
            </div>
          </div>
        </div>
      </Case>

      <Case label="selector · model, workspace and effort in one list">
        <div class="pt-palette">
          <div class="pt-output">
            <div class="relative">
              <Popover
                label={say("talk_choose")}
                columns={[
                  {
                    id: "model",
                    label: say("talk_column_model"),
                    items: MODELS.map((each) => ({
                      id: each.id,
                      label: each.id,
                      hint: "zenmux",
                      chosen: each.id === MODELS[1]?.id,
                    })),
                  },
                  {
                    id: "workspace",
                    label: say("talk_column_workspace"),
                    items: [
                      { id: "hall/mayor", label: "hall/mayor", chosen: true },
                      { id: "lab/east", label: "lab/east" },
                    ],
                  },
                  {
                    id: "effort",
                    label: say("talk_column_effort"),
                    items: EFFORTS.map((effort) => ({
                      id: effort,
                      label: say(`effort_${effort}`),
                      chosen: effort === "medium",
                    })),
                  },
                ]}
                onApply={() => undefined}
                onClose={() => undefined}
                bind={() => undefined}
              />
            </div>
          </div>
        </div>
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

      <Case label="machine · being checked">
        <MachineSkeleton />
      </Case>

      <Case label="machine · not checked yet">
        <MachineUnchecked onRecheck={() => undefined} />
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

      <Case label="settings · how hard the city thinks by default">
        <EffortChoice />
      </Case>

      <Case label="settings · where this city keeps its skills">
        <SkillsNote />
      </Case>
    </>
  );
}

// Every shared control, in each state it can be in. They are gathered in
// one component rather than spread through `Gallery` because the states
// that need a signal - a chosen model, a ticked row, an open dialog -
// are held here beside the fixtures that show them.
function Parts() {
  const say = useSay();
  const [host, setHost] = createSignal("api.zenmux.ai");
  const [bad, setBad] = createSignal("api_gateway.internal");
  const [key, setKey] = createSignal("secret:providers/zenmux");
  const [model, setModel] = createSignal<string | null>(null);
  const [rows, setRows] = createSignal<readonly ModelRow[]>(MODELS);
  const [picked, setPicked] = createSignal<readonly string[]>([MODELS[1]?.id ?? ""]);
  const [asking, setAsking] = createSignal(false);
  const [lens, setLens] = createSignal<string>("run_turns");

  const columns: readonly Column<ModelRow>[] = [
    {
      key: "id",
      header: say("part_model_id"),
      render: (row) => <span class="font-mono text-note">{row.id}</span>,
      compare: (a, b) => a.id.localeCompare(b.id),
    },
    {
      key: "context",
      header: say("part_context_window"),
      render: (row) => <span class="font-mono text-note">{row.context}</span>,
      compare: (a, b) => Number(a.context) - Number(b.context),
    },
    {
      key: "ceiling",
      header: say("part_output_ceiling"),
      render: (row) => <span class="font-mono text-note">{row.ceiling}</span>,
      editable: {
        text: (row) => row.ceiling,
        onEdit: (row, text) => {
          setRows(rows().map((each) => (each.id === row.id ? { ...each, ceiling: text } : each)));
        },
      },
    },
  ];

  return (
    <>
      <Case label="button · four tones">
        <div class="flex flex-wrap items-center gap-snug">
          <Button label={say("part_save")} tone="primary" />
          <Button label={say("part_cancel")} tone="secondary" />
          <Button label={say("dismiss")} tone="quiet" />
          <Button label={say("part_delete")} tone="destructive" />
        </div>
      </Case>

      <Case label="button · loading and refused">
        <div class="flex flex-wrap items-center gap-snug">
          <Button label={say("part_save")} tone="primary" loading />
          <Button label={say("part_save")} tone="primary" why={say("part_why_halted")} />
        </div>
      </Case>

      <Case label="field · help, error, mono with fixed parts">
        <div class="flex flex-col gap-base">
          <Field
            label={say("setup_base_url")}
            help={say("part_help_base_url")}
            value={host()}
            onInput={setHost}
            prefix="https://"
            suffix="/v1"
          />
          <Field
            label={say("setup_base_url")}
            error={say("part_error_host")}
            value={bad()}
            onInput={setBad}
          />
          <Field label={say("setup_key")} value={key()} onInput={setKey} mono />
        </div>
      </Case>

      <Case label="combobox · searchable, nothing chosen">
        <Combobox
          label={say("setup_models")}
          placeholder={say("part_search")}
          empty={say("part_no_match")}
          choices={MODELS.map((each) => ({ value: each.id, label: each.id, note: each.context }))}
          value={model()}
          onPick={setModel}
        />
      </Case>

      <Case label="table · ticked, sortable, corrected in place">
        <Table
          caption={say("setup_models")}
          columns={columns}
          rows={rows()}
          keyOf={(row) => row.id}
          selection={{
            picked: (row) => picked().includes(row.id),
            onPick: (row, on) => {
              setPicked(on ? [...picked(), row.id] : picked().filter((each) => each !== row.id));
            },
            allLabel: say("part_select_all"),
            allPicked: () => picked().length === rows().length,
            onPickAll: (on) => {
              setPicked(on ? rows().map((each) => each.id) : []);
            },
          }}
        />
      </Case>

      <Case label="table · empty">
        <Table
          caption={say("setup_models")}
          columns={columns}
          rows={[]}
          keyOf={(row) => row.id}
          empty={<EmptyState text={say("setup_no_models")} action={<Button label={say("setup_look")} tone="primary" />} />}
        />
      </Case>

      <Case label="row · status and actions">
        <div class="rounded-card border border-g3">
          <Row
            primary="hall/mayor"
            secondary={say("talk_tokens", { n: "12480" })}
            status={<Badge text={say("status_in_progress")} weight="live" dot />}
            actions={<Button label={say("talk_stop")} tone="quiet" />}
            onOpen={() => undefined}
          />
          <Row
            primary="lab/east"
            secondary={say("tree_no_runs")}
            status={<Badge text={say("city_quiet")} dot />}
          />
        </div>
      </Case>

      <Case label="banner · stopped, not connected, waiting">
        <div class="flex flex-col gap-snug">
          <Banner
            text={say("talk_halted")}
            detail={say("city_active", { n: "3" })}
            weight="alert"
            action={<Button label={say("talk_release")} tone="secondary" />}
          />
          <Banner
            text={say("talk_not_live")}
            weight="alert"
            action={<Button label={say("link_retry")} tone="secondary" />}
          />
          <Banner
            text={say("wait_title")}
            detail={say("nav_waiting", { n: "2" })}
            action={<Button label={say("wait_allow")} tone="primary" />}
          />
        </div>
      </Case>

      <Case label="notice · toast and entry in the centre">
        <div class="flex flex-col gap-snug">
          <Notice
            title={say("part_saved")}
            detail={say("setup_base_url")}
            at="03:35"
            action={<Button label={say("part_undo")} tone="quiet" />}
            dismiss={<Button label={say("dismiss")} tone="quiet" />}
          />
          <div class="rounded-card border border-g3">
            <Notice seat="entry" title={say("part_saved")} at="03:35" />
            <Notice seat="entry" weight="alert" title={say("link_refused")} detail={say("part_error_host")} at="03:31" />
          </div>
        </div>
      </Case>

      <Case label="dialog · confirming what cannot be undone">
        <Button
          label={say("setup_remove")}
          tone="destructive"
          onPress={() => {
            setAsking(true);
          }}
        />
        <Dialog
          open={asking()}
          title={say("part_remove_endpoint")}
          detail={say("part_remove_detail")}
          confirmLabel={say("part_delete")}
          cancelLabel={say("part_cancel")}
          destructive
          onConfirm={() => {
            setAsking(false);
          }}
          onCancel={() => {
            setAsking(false);
          }}
        />
      </Case>

      <Case label="progress · counted and uncounted">
        <div class="flex flex-col gap-base">
          <Progress label={say("machine_ready")} done={4} total={6} />
          <Progress label={say("machine_ready")} done={0} total={0} />
        </div>
      </Case>

      <Case label="empty state">
        <EmptyState
          text={say("city_no_buildings")}
          action={<Button label={say("part_new_building")} tone="primary" />}
        />
      </Case>

      <Case label="skeleton · prose and rows">
        <div class="flex flex-col gap-wide">
          <Skeleton label={say("part_loading")} rows={3} />
          <Skeleton label={say("part_loading")} rows={4} tall />
        </div>
      </Case>

      <Case label="tabs · the lenses of one run">
        <Tabs
          label={say("run_lenses")}
          lenses={LENSES.map((key) => ({
            id: key,
            label: say(key),
            mark: key === "run_changes" ? <Badge text="7" /> : undefined,
          }))}
          current={lens()}
          onPick={setLens}
        />
      </Case>

      <Case label="badge · count and state">
        <div class="flex flex-wrap items-center gap-snug">
          <Badge text={say("nav_waiting", { n: "3" })} />
          <Badge text={say("status_in_progress")} weight="live" dot />
          <Badge text={say("status_blocked")} weight="alert" dot />
        </div>
      </Case>
    </>
  );
}
