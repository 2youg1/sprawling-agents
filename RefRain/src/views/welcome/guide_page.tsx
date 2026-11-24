// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { createEffect, createMemo, createSignal, createUniqueId, flush, For, Show } from "solid-js";
import { readAnswer } from "../../core/answered";
import { QUERIES } from "../../core/asking";
import { putGuide } from "../../core/commands";
import { say } from "../../core/lang";
import { MAYOR } from "../../core/route";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { GuideProgress, GuideStep } from "../../wire";
import Button from "../parts/button.solid";
import Page from "../parts/page.solid";
import type { Rank } from "../parts/page";
import Unanswered from "../parts/unanswered.solid";
import { outstanding } from "../setup/dependencies";
import {
  advanced,
  currentOf,
  left,
  opened,
  putOff,
  skipAll,
  standingOf,
  STEPS,
  type Configured,
} from "./guide";
import { ABOUT, bodyId, stepLookOf } from "./step";
import Step from "./step.look.solid";

export interface GuidePageProps {
  readonly rank?: Rank | undefined;
  readonly step?: GuideStep | undefined;
  readonly back?: JSX.Element;
  readonly body: (step: GuideStep) => JSX.Element;
}

function named(value: string | null | undefined): boolean {
  return value != null && value.trim() !== "";
}

export default function GuidePage(props: GuidePageProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const uid = createUniqueId();
  const endpoints = storeValue(u.conn.asking.ask(QUERIES.endpoints));
  const doctor = storeValue(u.conn.asking.ask(QUERIES.doctor));
  const identity = storeValue(u.conn.asking.ask(QUERIES.identity));
  const guide = storeValue(u.conn.asking.ask(QUERIES.guide));

  const configured = createMemo<Configured>(() => {
    const provider = endpoints();
    const machine = doctor();
    const names = identity();
    const chosen =
      provider !== undefined && "endpoints" in provider ? provider.endpoints.chosen : null;
    const missing =
      machine !== undefined && "doctor" in machine ? outstanding(machine.doctor, "use") : null;
    const stated =
      names !== undefined && "identity" in names && "stated" in names.identity
        ? names.identity.stated
        : null;
    return {
      provider: chosen === null ? null : chosen.some((entry) => entry.tag === "main"),
      dependencies: missing === null ? null : missing.length === 0,
      texts: stated === null ? null : named(stated.user_id) || named(stated.mayor),
      skills: null,
      mcp: null,
    };
  });
  const read = createMemo(() =>
    readAnswer(guide(), (held) => ("guide" in held ? held.guide : undefined)),
  );
  const lost = () => {
    const held = read();
    return held.kind === "unavailable" ? held : undefined;
  };
  const [sent, setSent] = createSignal<GuideProgress | null>(null);
  const [folded, setFolded] = createSignal(false);
  const [linked, setLinked] = createSignal<GuideStep | null>(null);
  const progress = createMemo<GuideProgress>(() => {
    const held = read();
    return sent() ?? (held.kind === "held" ? held.value : {});
  });
  const current = createMemo(() => currentOf(progress(), configured()));
  const open = () => (folded() ? null : (linked() ?? current()));
  const ready = () => configured().provider === true;

  function headId(step: GuideStep): string {
    return `${uid}-${step}`;
  }
  function focusStep(step: GuideStep): void {
    queueMicrotask(() => {
      flush();
      document.getElementById(headId(step))?.focus();
    });
  }

  createEffect(guide, (answer) => {
    if (answer !== undefined) setSent(null);
  });
  createEffect(
    () => props.step,
    (next) => {
      setLinked(next ?? null);
      setFolded(false);
      if (next !== undefined) focusStep(next);
    },
  );
  let before: Configured | null = null;
  createEffect(configured, (after) => {
    const was = before;
    const shown = open();
    before = after;
    if (was === null || shown === null) return;
    const next = advanced(progress(), shown, was, after);
    if (next !== null) {
      setLinked(null);
      setFolded(false);
      write(next);
    }
  });

  function write(next: GuideProgress): void {
    setSent(next);
    u.send(putGuide(next));
    u.conn.asking.refresh(QUERIES.guide);
  }
  function toggle(step: GuideStep): void {
    const shown = open();
    setLinked(null);
    if (shown === step) {
      setFolded(true);
      return;
    }
    setFolded(false);
    write(opened(progress(), step));
  }
  function later(step: Exclude<GuideStep, "provider">): void {
    setFolded(false);
    setLinked(null);
    const next = putOff(progress(), step);
    write(next);
    focusStep(next.at ?? step);
  }
  function leave(next: GuideProgress): void {
    write(next);
    u.prefs.setWelcomed(true);
    u.go({ kind: "talk", address: MAYOR });
  }

  return (
    <div class="flex min-w-0 flex-1 flex-col @min-[64rem]:silver-columns">
      <div class="flex min-w-0 flex-1 flex-col @min-[64rem]:col-start-2">
        {props.back}
        <Page
          title={say(lang(), "welcome_title")}
          note={say(lang(), "welcome_note")}
          rank={props.rank}
        >
          <div class="@container min-w-0">
            <div class="flex min-w-0 flex-col">
              <Show when={lost()}>
                {(failure) => <Unanswered query={failure().query} asked={QUERIES.guide} />}
              </Show>
              <ol aria-label={say(lang(), "guide_steps")}>
                <For each={STEPS}>
                  {(step, index) => {
                    const header = () =>
                      stepLookOf(
                        {
                          step,
                          at: index(),
                          standing: standingOf(step, progress(), configured()),
                          open: open() === step,
                          id: headId(step),
                          lang: lang(),
                        },
                        () => toggle(step),
                      );
                    return (
                      <li class="border-b border-edge">
                        <Step {...header()} routeFocus={props.step === step && open() === step} />
                        <Show when={open() === step}>
                          <section
                            id={bodyId(headId(step))}
                            aria-labelledby={headId(step)}
                            class="drop flex min-w-0 flex-col gap-base pb-wide pl-[calc(4ch+var(--spacing-base))] @max-[40rem]:pl-0"
                          >
                            <p class="text-note text-text-quiet">{say(lang(), ABOUT[step])}</p>
                            {props.body(step)}
                            {step === "provider" ? null : (
                              <div class="flex justify-end">
                                <Button
                                  label={say(lang(), "guide_later")}
                                  tone="quiet"
                                  onPress={() => later(step)}
                                />
                              </div>
                            )}
                          </section>
                        </Show>
                      </li>
                    );
                  }}
                </For>
              </ol>
              <div class="flex flex-wrap items-center gap-base pt-wide">
                <Button
                  label={say(lang(), "guide_start")}
                  tone="primary"
                  {...(ready() ? {} : { why: say(lang(), "guide_start_needs_main") })}
                  onPress={() => leave(left(progress()))}
                />
                <Button
                  label={say(lang(), "guide_put_off_rest")}
                  tone="quiet"
                  onPress={() => leave(skipAll(progress()))}
                />
              </div>
            </div>
          </div>
        </Page>
      </div>
    </div>
  );
}
