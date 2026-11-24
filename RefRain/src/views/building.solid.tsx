// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, Match, Show, Switch } from "solid-js";
import { readAnswer } from "../core/answered";
import { within } from "../core/belief/live";
import { halt, release, removeBuilding } from "../core/commands";
import { fill, say, type Key } from "../core/lang";
import { removalOf } from "../core/removal";
import { buildingIsShut } from "../core/scope";
import { storeValue } from "../core/solid_store";
import { ui } from "../ui";
import type { Address, Query } from "../wire";
import Catalog from "./building/catalog.solid";
import Commits from "./building/commits.solid";
import Directory from "./building/directory.solid";
import FileView from "./building/file_view";
import Goal from "./building/goal.solid";
import Plan from "./building/plan.solid";
import Prompts from "./building/prompts.solid";
import Rooms from "./building/rooms.solid";
import Sandbox from "./building/sandbox.solid";
import { sectionsLookOf } from "./building/sections";
import Sections from "./building/sections.look.solid";
import Status from "./building/status.solid";
import Tree, { type Picked } from "./building/tree.solid";
import { inspectorItems, openItems, type RightItem } from "./inspect/open";
import Button from "./parts/button.solid";
import Dialog from "./parts/dialog_view";
import Page from "./parts/page.solid";
import type { Rank } from "./parts/page";
import Unanswered from "./parts/unanswered.solid";
import Right from "./right";
import WorkspaceNav from "./shared/workspace.solid";

const SECTIONS = ["plan", "commits", "changes", "prompts", "catalog", "sandbox"] as const;
type Section = (typeof SECTIONS)[number];

const SECTION_WORD: Record<Section, Key> = {
  plan: "bld_plan",
  commits: "bld_commits",
  changes: "bld_changes",
  prompts: "bld_prompts",
  catalog: "bld_catalog",
  sandbox: "bld_sandbox",
};

const WAITING = "…";

export interface BuildingProps {
  readonly address: Address;
  readonly rank?: Rank | undefined;
  readonly beside?: readonly RightItem[] | undefined;
  readonly initialSection?: "catalog" | undefined;
}

export default function BuildingPage(props: BuildingProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const items = storeValue(inspectorItems);
  const [section, setSection] = createSignal<Section | null>("plan");
  const [picked, setPicked] = createSignal<Picked | null>(null);
  const [removing, setRemoving] = createSignal(false);

  createEffect(
    () => props.initialSection,
    (next) => {
      if (next !== undefined) setSection(next);
    },
  );

  const question = createMemo<Query>(() => ({ building_view: { addr: props.address } }));
  const asked = storeValue(() => u.conn.asking.ask(question()));
  const read = createMemo(() =>
    readAnswer(asked(), (answer) => ("building" in answer ? answer.building : undefined)),
  );
  const building = () => {
    const answer = read();
    return answer.kind === "held" ? answer.value : undefined;
  };
  const unavailable = () => {
    const answer = read();
    return answer.kind === "unavailable" ? answer : undefined;
  };
  const catalog = storeValue(() => u.conn.asking.ask({ catalog: { building: props.address } }));
  const changed = () => {
    const answer = catalog();
    return answer !== undefined && "catalog" in answer ? answer.catalog.changed : 0;
  };
  const done = () => {
    const held = building();
    return held !== undefined && "planned" in held.progress && held.progress.planned.total > 0
      ? held.progress.planned
      : null;
  };
  const halted = () => buildingIsShut(belief().halted, props.address);
  const opened = () => openItems(items()).length > 0 || (props.beside?.length ?? 0) > 0;
  const removal = () => removalOf(props.address, livingIn(props.address));
  const shown = () => {
    const at = section();
    return at === null ? (picked()?.at ?? "") : say(lang(), SECTION_WORD[at]);
  };

  function livingIn(room: Address): number {
    return belief().live.filter((run) => within(run, room)).length;
  }

  function pick(next: Picked): void {
    setPicked(next);
    setSection(null);
  }

  const index = createMemo(() =>
    sectionsLookOf(
      SECTIONS.map((key) => ({
        key,
        label:
          key === "catalog" && changed() > 0
            ? fill(say(lang(), "catalog_nav_changed"), { n: String(changed()) })
            : say(lang(), SECTION_WORD[key]),
      })),
      section(),
      (next) => {
        setSection(next);
        setPicked(null);
      },
    ),
  );

  return (
    <>
      <Page
        title={props.address}
        rank={props.rank}
        aside={
          <>
            <Show when={done()}>
              {(progress) => (
                <span class="figure text-note text-text-quiet">
                  {fill(say(lang(), "bld_progress"), {
                    done: String(progress().done),
                    total: String(progress().total),
                  })}
                </span>
              )}
            </Show>
            <Show when={!halted() && livingIn(props.address) === 0}>
              <span class="text-note text-text-faint">{say(lang(), "bld_halt_idle")}</span>
            </Show>
            <Button
              label={fill(say(lang(), halted() ? "bld_release" : "bld_halt"), {
                addr: props.address,
              })}
              tone={halted() ? "secondary" : "quiet"}
              onPress={() => {
                const scope = { building: props.address };
                u.send(halted() ? release(scope) : halt(scope));
              }}
            />
            <Show when={removal() !== "hall"}>
              <Button
                label={say(lang(), "bld_remove")}
                tone="quiet"
                {...(removal() === "busy" ? { why: say(lang(), "bld_remove_busy") } : {})}
                onPress={() => setRemoving(true)}
              />
            </Show>
          </>
        }
      >
        <WorkspaceNav current={props.address} />
        <Goal address={props.address} />
        <div class="grid grid-cols-11 grid-rows-[auto_1fr] items-start gap-x-gutter gap-y-wide narrow:grid-cols-1 narrow:grid-rows-none">
          <nav
            class="col-[1/4] row-start-1 min-w-0 narrow:col-span-full narrow:row-auto"
            aria-label={say(lang(), "bld_index")}
          >
            <Sections {...index()} />
          </nav>
          <section
            class={[
              "row-[1/3] flex min-w-0 flex-col narrow:col-span-full narrow:row-auto",
              opened() ? "col-[4/8]" : "col-[4/12]",
            ]}
            aria-label={shown()}
          >
            <Switch>
              <Match when={section() === "plan"}>
                <Switch fallback={<p class="text-text-faint">{WAITING}</p>}>
                  <Match when={building()}>{(held) => <Plan answer={held()} />}</Match>
                  <Match when={unavailable()}>
                    {(lost) => <Unanswered query={lost().query} asked={question()} />}
                  </Match>
                </Switch>
              </Match>
              <Match when={section() === "commits"}>
                <Commits building={props.address} />
              </Match>
              <Match when={section() === "changes"}>
                <Status building={props.address} />
              </Match>
              <Match when={section() === "prompts"}>
                <Prompts building={props.address} rooms={building()?.rooms ?? []} />
              </Match>
              <Match when={section() === "catalog"}>
                <Catalog building={props.address} />
              </Match>
              <Match when={section() === "sandbox"}>
                <Sandbox
                  address={props.address}
                  held={building()?.sandbox ?? null}
                  known={building() !== undefined}
                />
              </Match>
              <Match when={picked()?.kind === "file" && picked()}>
                {(file) => <FileView at={file().at} root={props.address} />}
              </Match>
              <Match when={picked()?.kind === "directory" && picked()}>
                {(dir) => <Directory at={dir().at} root={props.address} onPick={pick} />}
              </Match>
            </Switch>
          </section>
          <Show when={opened()}>
            <div class="sticky top-0 col-[8/12] row-[1/3] flex h-[calc(100dvh-var(--spacing-section)*4)] min-w-0 overflow-hidden rounded-panel border border-edge narrow:col-span-full narrow:row-auto">
              <Right following={props.beside ?? []} current={undefined} talk={props.address} />
            </div>
          </Show>
          <div class="col-[1/4] row-start-2 flex min-w-0 flex-col gap-wide narrow:col-span-full narrow:row-auto">
            <Tree root={props.address} picked={picked()} onPick={pick} />
            <Show when={building()}>
              {(answer) => (
                <Rooms
                  answer={answer()}
                  living={livingIn}
                  onPick={(room) => pick({ at: room, kind: "directory" })}
                />
              )}
            </Show>
          </div>
        </div>
      </Page>
      <Dialog
        open={removing()}
        title={fill(say(lang(), "bld_remove_title"), { addr: props.address })}
        detail={say(lang(), "bld_remove_detail")}
        confirmLabel={say(lang(), "bld_remove")}
        cancelLabel={say(lang(), "part_cancel")}
        destructive
        onConfirm={() => {
          setRemoving(false);
          if (u.send(removeBuilding(props.address))) u.go({ kind: "city" });
        }}
        onCancel={() => setRemoving(false)}
      />
    </>
  );
}
