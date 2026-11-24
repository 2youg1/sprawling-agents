// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, For, Match, Show, Switch } from "solid-js";
import { fill, say, type Key, type Lang } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { DoctorAnswer, DoctorCore, DoctorItem, DoctorNewest, DoctorTier } from "../../wire";
import Badge from "../parts/badge.solid";
import Button from "../parts/button.solid";
import Copy from "../parts/copy.solid";
import type { Weight } from "../parts/glyph";
import Progress from "../parts/progress.solid";
import {
  absenceOf,
  enablesKey,
  ofTier,
  outstanding,
  rowOf,
  siteOf,
  sourceOf,
  standing,
  type Absence,
} from "../setup/dependencies";
import { over, type StepState, type Walk } from "../setup/installing";
import DoctorLink from "./doctor_link.solid";
import Pack from "./pack.solid";
import Scanning from "./scanning.solid";
import Versions from "./versions.solid";

export interface MachineReportProps {
  readonly answer: DoctorAnswer;
  readonly onInstall?: ((item: string) => void) | undefined;
  readonly planned?: readonly string[] | undefined;
  readonly onInstallAll?: (() => void) | undefined;
  readonly walk?: Walk | null | undefined;
  readonly newest?: Readonly<Record<string, DoctorNewest>> | undefined;
  readonly onInstallPack?: ((names: readonly string[]) => void) | undefined;
}

function weightOf(item: DoctorItem, absence: Absence): Weight {
  if ("present" in item.state) return "quiet";
  if ("broken" in item.state) return "alert";
  return item.need === "optional" || absence === "spare" ? "quiet" : "alert";
}

function coreOf(core: DoctorCore): { readonly key: Key; readonly said: string | null } {
  if (typeof core !== "string") {
    return "refused" in core
      ? { key: "machine_core_refused", said: core.refused.said }
      : { key: "machine_core_unasked", said: core.unasked.said };
  }
  switch (core) {
    case "raised":
      return { key: "machine_core_raised", said: null };
    case "held_by_setting":
      return { key: "machine_core_held", said: null };
    case "lowered_by_valve":
      return { key: "machine_core_lowered", said: null };
  }
}

const STEP: Record<StepState, { readonly key: Key; readonly weight: Weight }> = {
  waiting: { key: "machine_step_waiting", weight: "quiet" },
  running: { key: "machine_step_running", weight: "quiet" },
  done: { key: "machine_step_done", weight: "quiet" },
  failed: { key: "machine_step_failed", weight: "alert" },
};

function installNote(item: DoctorItem, how: string, lang: Lang): string | null {
  const install = item.install;
  if (typeof install === "string" || "manual" in install) return null;
  if ("command" in install) {
    const source = sourceOf(how);
    return fill(say(lang, "machine_install_press"), {
      source: "key" in source ? say(lang, source.key) : source.program,
    });
  }
  const site = siteOf(how);
  return site === null
    ? say(lang, "machine_install_admin")
    : fill(say(lang, "machine_install_by_hand"), { site });
}

function DoctorRow(props: {
  readonly item: DoctorItem;
  readonly absence: Absence;
  readonly lang: Lang;
  readonly newest: DoctorNewest | undefined;
  readonly walk: Walk | null | undefined;
  readonly onInstall: MachineReportProps["onInstall"];
}) {
  const line = createMemo(() => rowOf(props.item));
  const enables = () => enablesKey(props.item.name);
  const word = (key: Key) => say(props.lang, key);
  const note = () => {
    const how = line().install;
    return how === null ? null : installNote(props.item, how, props.lang);
  };
  return (
    <li class="flex min-w-0 flex-col gap-tight border-t border-edge pt-snug pb-base">
      <div class="flex min-w-0 items-center gap-snug">
        <DoctorLink item={props.item} />
        <Badge text={word(line().state)} weight={weightOf(props.item, props.absence)} dot />
        <Switch>
          <Match when={props.item.need === "optional"}>
            <span class="shrink-0 text-note text-text-faint">{word("machine_optional")}</span>
          </Match>
          <Match when={props.absence === "spare"}>
            <span class="min-w-0 truncate text-note text-text-faint">{word("machine_spare")}</span>
          </Match>
        </Switch>
      </div>
      <Versions item={props.item} newest={props.newest} />
      <Show when={line().reason}>
        {(why) => (
          <p class="flex min-w-0 flex-wrap items-baseline gap-tight text-note">
            <span class="text-text-quiet">{word(why().key)}</span>
            <Show when={why().said}>
              {(said) => <span class="min-w-0 break-all font-mono text-text-faint">{said()}</span>}
            </Show>
          </p>
        )}
      </Show>
      <Show when={enables()}>
        {(entry) => <p class="text-note text-text-faint">{word(entry())}</p>}
      </Show>
      <Show when={line().offer !== "held"}>
        <Show
          when={line().install}
          fallback={<span class="text-note text-text-faint">{word("machine_no_recipe")}</span>}
        >
          {(how) => (
            <>
              <div class="flex min-w-0 items-start gap-tight">
                <code class="min-w-0 flex-1 whitespace-pre-wrap break-words rounded-control bg-chrome px-snug py-tight font-mono text-note text-text-quiet">
                  <For each={how().split(" ")} keyed={false}>
                    {(segment, index) => (
                      <>
                        {index > 0 ? " " : ""}
                        <span class="inline-block max-w-full">{segment()}</span>
                      </>
                    )}
                  </For>
                </code>
                <Show when={line().offer === "press"}>
                  <Button
                    label={word("machine_install")}
                    tone="secondary"
                    loading={
                      props.walk?.some(
                        (step) => step.name === props.item.name && step.state === "running",
                      ) === true
                    }
                    onPress={() => props.onInstall?.(props.item.name)}
                  />
                </Show>
                <Copy text={how()} />
              </div>
              <Show when={note()}>
                {(hint) => <p class="text-note text-text-faint">{hint()}</p>}
              </Show>
            </>
          )}
        </Show>
      </Show>
    </li>
  );
}

function InstallSteps(props: { readonly walk: Walk; readonly lang: Lang }) {
  const done = () =>
    props.walk.filter((step) => step.state === "done" || step.state === "failed").length;
  const title = () => say(props.lang, "machine_walk_progress");
  return (
    <div class="flex min-w-0 flex-col gap-snug border-y border-edge py-snug">
      <span class="text-note text-text-quiet">{title()}</span>
      <Progress label={title()} done={done()} total={props.walk.length} />
      <ol class="flex min-w-0 flex-col gap-tight">
        <For each={props.walk} keyed={(step) => step.name}>
          {(step) => (
            <li class="flex min-w-0 flex-wrap items-baseline gap-snug text-note">
              <span class="w-[10rem] shrink-0 truncate font-mono text-text">{step().name}</span>
              <Badge
                text={say(props.lang, STEP[step().state].key)}
                weight={STEP[step().state].weight}
                dot
              />
              <Show when={step().why}>
                {(why) => <span class="min-w-0 flex-1 break-words text-text-faint">{why()}</span>}
              </Show>
            </li>
          )}
        </For>
      </ol>
    </div>
  );
}

function Tier(props: MachineReportProps & { readonly tier: DoctorTier; readonly lang: Lang }) {
  const word = (key: Key) => say(props.lang, key);
  const title = () => word(props.tier === "use" ? "machine_tier_use" : "machine_tier_develop");
  const items = createMemo(() => ofTier(props.answer, props.tier));
  const packed = createMemo(() => items().filter((item) => item.pack === "rust_tools"));
  const missing = createMemo(() => outstanding(props.answer, props.tier));
  const far = createMemo(() => standing(items(), missing()?.length ?? null));
  const walking = () => props.walk != null && !over(props.walk);
  const planned = () => props.planned?.length ?? 0;
  return (
    <section class="flex min-w-0 flex-col gap-snug" aria-label={title()}>
      <div class="flex min-w-0 flex-wrap items-center gap-base">
        <h2 class="text-label font-label text-text">{title()}</h2>
        <Show when={props.tier === "develop" && planned() > 0 && props.onInstallAll}>
          {(install) => (
            <>
              <Button
                label={fill(word("machine_install_all"), { count: String(planned()) })}
                tone="primary"
                loading={walking()}
                onPress={install()}
              />
              <span class="text-note text-text-faint">{word("machine_install_all_note")}</span>
            </>
          )}
        </Show>
      </div>
      <Progress label={word("machine_progress_label")} done={far().done} total={far().total} />
      <Show when={(missing()?.length ?? 0) > 0}>
        <p class="flex flex-wrap items-baseline gap-tight text-note">
          <span class="text-text-quiet">{word("machine_missing")}</span>
          <For each={missing() ?? []}>
            {(item) => <span class="font-mono text-alert">{item}</span>}
          </For>
        </p>
      </Show>
      <Show when={props.tier === "develop" && props.walk}>
        {(walk) => <InstallSteps walk={walk()} lang={props.lang} />}
      </Show>
      <ul class="grid grid-cols-[repeat(auto-fill,minmax(340px,1fr))] items-start gap-x-gutter">
        <For each={items().filter((item) => item.pack == null)} keyed={(item) => item.name}>
          {(item) => (
            <DoctorRow
              item={item()}
              absence={absenceOf(item(), missing(), items())}
              lang={props.lang}
              newest={props.newest?.[item().name]}
              walk={props.walk}
              onInstall={props.onInstall}
            />
          )}
        </For>
        <Show when={packed().length > 0}>
          <Pack
            members={packed()}
            newest={props.newest ?? {}}
            walking={walking()}
            onInstall={props.onInstallPack}
          />
        </Show>
      </ul>
    </section>
  );
}

export default function MachineReport(props: MachineReportProps) {
  const lang = storeValue(ui().lang);
  const core = createMemo(() => coreOf(props.answer.core));
  return (
    <div class="flex min-w-0 flex-col gap-wide">
      <Tier {...props} tier="use" lang={lang()} />
      <Tier {...props} tier="develop" lang={lang()} />
      <p class="flex min-w-0 flex-wrap items-baseline gap-tight text-note">
        <span class="text-text-quiet">{say(lang(), "machine_core")}</span>
        <span class="text-text">{say(lang(), core().key)}</span>
        <Show when={core().said}>
          {(said) => <span class="min-w-0 break-words text-text-faint">{said()}</span>}
        </Show>
      </p>
      <Scanning scanning={props.answer.scanning} />
    </div>
  );
}
