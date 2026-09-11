// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What this machine has, as the city found it when it started. The
// first step of the first half hour used to be one command to copy and
// no way of knowing whether it had worked; this is the answer.

import { For, Show, createMemo } from "solid-js";

import type { DoctorAnswer, DoctorInstall, DoctorItem, DoctorState } from "../wire";
import { useSay, useUi } from "../ui";

const DOCTOR = "sprawling doctor --install";

// The one word a row is labelled by. The state is a value the city
// sent; every word around it comes from the phrase table.
function stateKey(
  state: DoctorState,
): "machine_present" | "machine_broken" | "machine_absent" {
  if ("present" in state) return "machine_present";
  if ("broken" in state) return "machine_broken";
  return "machine_absent";
}

// What a present item said when asked its version, when it said
// anything a person can read. The three wordless answers - it said
// nothing, its line was not text, it was late - are facts about how it
// did not answer, and a row shows the item rather than them.
function versionOf(state: DoctorState): string | null {
  if (!("present" in state)) return null;
  const said = state.present.version;
  return typeof said === "object" ? said.said.text : null;
}

// The command that would get a missing item, as a person would type it.
function spelledOf(install: DoctorInstall): string | null {
  if (typeof install === "string") return null;
  if ("command" in install) return install.command.spelled;
  if ("print" in install) return install.print.spelled;
  return install.manual.how;
}

function Row(props: { readonly item: DoctorItem }) {
  const say = useSay();
  const version = createMemo(() => versionOf(props.item.state));
  const spelled = createMemo(() => spelledOf(props.item.install));
  const here = createMemo(() => "present" in props.item.state);
  return (
    <li class="flex items-baseline gap-base py-tight">
      <span
        class={`w-step shrink-0 text-center text-note ${here() ? "text-accent" : props.item.need === "optional" ? "text-text-disabled" : "text-alert"}`}
      >
        {say(stateKey(props.item.state))}
      </span>
      <span class="w-tree shrink-0 truncate font-mono text-label text-text">{props.item.name}</span>
      <Show when={props.item.need === "optional"}>
        <span class="shrink-0 text-note text-text-disabled">{say("machine_optional")}</span>
      </Show>
      <span class="min-w-0 flex-1 truncate text-note text-text-faint">
        <Show when={version()} fallback={<Show when={spelled()}>{(how) => <code class="font-mono">{how()}</code>}</Show>}>
          {(said) => said()}
        </Show>
      </span>
    </li>
  );
}

function Verdicts(props: { readonly answer: DoctorAnswer }) {
  const say = useSay();
  return (
    <ul class="mb-base flex flex-col gap-tight text-label" aria-label={say("machine_verdicts")}>
      <For each={props.answer.tiers}>
        {(each) => (
          <li class="flex items-baseline gap-base">
            <span class="w-tree shrink-0 text-text-quiet">{say(`machine_tier_${each.tier}`)}</span>
            <Show
              when={each.missing.length > 0}
              fallback={<span class="text-accent">{say("machine_ready")}</span>}
            >
              <span class="min-w-0 flex-1 truncate font-mono text-alert">{each.missing.join(" ")}</span>
            </Show>
          </li>
        )}
      </For>
    </ul>
  );
}

// One answer, drawn. Separate from the ask so the gallery can show
// this machine in states nobody's own machine happens to be in.
export function MachineReport(props: { readonly answer: DoctorAnswer }) {
  const say = useSay();
  return (
    <>
      <Verdicts answer={props.answer} />
      <ul class="mb-wide border-t border-line" aria-label={say("machine_items")}>
        <For each={props.answer.items}>{(each) => <Row item={each} />}</For>
      </ul>
    </>
  );
}

// This machine, item by item, with the one command that changes it.
export function Machine() {
  const ui = useUi();
  const say = useSay();
  const held = ui.conn.asking.ask("doctor");
  const answer = createMemo(() => {
    const now = held();
    return now !== undefined && "doctor" in now ? now.doctor : undefined;
  });
  return (
    <div>
      <Show when={answer()} fallback={<p class="mb-base text-note text-text-faint">{say("machine_unasked")}</p>}>
        {(found) => <MachineReport answer={found()} />}
      </Show>
      <p class="mb-base text-note text-text-faint">{say("welcome_machine_body")}</p>
      <div class="flex items-center gap-snug">
        <code class="rounded-control bg-g1 px-base py-snug font-mono text-text">{DOCTOR}</code>
        <button
          type="button"
          class="rounded-control px-snug py-tight text-label text-text-quiet hover:bg-g1"
          onClick={() => {
            void navigator.clipboard.writeText(DOCTOR);
          }}
        >
          {say("setup_copy")}
        </button>
      </div>
    </div>
  );
}
