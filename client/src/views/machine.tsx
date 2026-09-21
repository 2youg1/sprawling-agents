// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What this machine has, as the city found it when it started. The
// first step of the first half hour used to be one command to copy and
// no way of knowing whether it had worked; this is the answer.
//
// Two columns, because a person acts differently on the two: what the
// city cannot run without, and what it would use if the machine had it.
// Each column carries how far it has got; each row carries the state,
// the name, the version, one action, and the whole command - wrapped,
// never cut, because a command cut at the right edge cannot be typed.
//
// One click installs. `Command::DoctorInstall` runs the recipe this
// city may run and refuses the other two with what a person does
// instead, and the install ends by looking at this machine again - so
// `check again` is the manual half of the same verb rather than a
// second way of getting the same answer.

import type { Accessor } from "solid-js";
import { For, Match, Show, Switch, createEffect, createMemo, createSignal, on } from "solid-js";

import { QUERIES } from "../core/asking";
import { doctorInstall, doctorRefresh } from "../core/commands";
import type { Key } from "../core/lang";
import type {
  Answer,
  DoctorAnswer,
  DoctorInstall,
  DoctorItem,
  DoctorState,
  DoctorTier,
} from "../wire";
import { useSay, useUi } from "../ui";
import { Button } from "./parts/button";
import type { ButtonProps, Tone } from "./parts/button";
import { Progress } from "./parts/progress";
import { Tip } from "./parts/tip";

const DOCTOR = "sprawling doctor --install";

// How a person updates, spelled the two ways they may have installed.
// Neither is run from here: where this binary lives belongs to whoever
// put it there, so the page prints the command and stops.
const UPDATE_NPM = "bunx sprawling@latest up";
const RELEASES = "https://github.com/2youg1/sprawling-agents/releases";

// The one word a row is labelled by. The state is a value the city
// sent; every word around it comes from the phrase table.
function stateKey(
  state: DoctorState,
): "machine_present" | "machine_broken" | "machine_absent" {
  if ("present" in state) return "machine_present";
  if ("broken" in state) return "machine_broken";
  return "machine_absent";
}

// Colour is never the only signal: the dot repeats what the word beside
// it already says, for the eye that scans a column rather than reads it.
function dotOf(item: DoctorItem): string {
  if ("present" in item.state) return "bg-accent";
  if ("broken" in item.state) return "bg-alert";
  return item.need === "optional" ? "bg-mark" : "bg-alert";
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

// What the city cannot run without. Everything else - the optional
// members of the run tier and the whole develop tier - is a
// recommendation rather than a requirement.
function required(answer: DoctorAnswer): readonly DoctorItem[] {
  return answer.items.filter((each) => each.tier === "use" && each.need === "required");
}

function recommended(answer: DoctorAnswer): readonly DoctorItem[] {
  return answer.items.filter((each) => each.tier !== "use" || each.need !== "required");
}

// What the city still names as missing for one tier, or nothing when
// it has answered about no such tier.
//
// The names rather than the count of them: the column states both, and
// a count taken here and a list taken somewhere else would be two
// readings of one verdict.
function outstanding(answer: DoctorAnswer, tier: DoctorTier): readonly string[] | null {
  return answer.tiers.find((each) => each.tier === tier)?.missing ?? null;
}

// How far a column has got, counted the way the city counts it.
//
// **The rows are not the authority for what is missing.** A tier's
// verdict collapses a run of interchangeable items - any one browser
// engine will do - into the single thing a person is still missing, so
// a machine with Firefox on it is missing no browser however many of
// the other rows stay empty; counting rows was what kept that column
// short of full for somebody who had everything.
//
// The denominator is therefore what this machine has, plus what the
// city still names for the tier, plus the optional rows it does not
// have: an optional item counts against no tier, and is still a row a
// person can act on. The bar fills exactly when nothing on the column
// is left to do.
//
// A tier the city said nothing about leaves the end unknown, and
// `Progress` draws that as busy rather than as a fraction.
function standing(
  items: readonly DoctorItem[],
  missing: number | null,
): { readonly done: number; readonly total: number } {
  const has = (item: DoctorItem) => "present" in item.state;
  const done = items.filter(has).length;
  const wanted = items.filter((each) => !has(each) && each.need === "optional").length;
  return { done, total: missing === null ? 0 : done + missing + wanted };
}

// Whether this city may run the install itself. The other two recipes
// are a command the person runs and an instruction they follow, and
// both stay a copy rather than a button.
function runnable(install: DoctorInstall): boolean {
  return typeof install !== "string" && "command" in install;
}

// What the install control offers for one item.
//
// One reading of the item, so the colour, the reason and the press can
// never disagree: what this machine already has is not offered in the
// colour reserved for the one action a screen is for.
type Offer = "press" | "by_hand" | "held";

function offerOf(item: DoctorItem): Offer {
  if ("present" in item.state) return "held";
  return runnable(item.install) ? "press" : "by_hand";
}

// The paint each offer carries, and the reason it carries when a press
// does nothing. A reason is what makes the control grey - `Button`
// owns that rule - so this table states the reason and never the grey.
const OFFER: Record<Offer, { readonly tone: Tone; readonly why: Key | null }> = {
  press: { tone: "primary", why: null },
  by_hand: { tone: "quiet", why: "machine_install_by_hand" },
  held: { tone: "quiet", why: "machine_installed" },
};

// One item, and everything a person decides about it from one line: the
// state, what it is, which version answered, what it enables, and the
// command that would get it.
//
// The name links to the item's own site when the city knows one, so a
// person can read what a thing is before installing it.
function Row(props: { readonly item: DoctorItem; readonly onInstall: (item: string) => void }) {
  const say = useSay();
  const version = createMemo(() => versionOf(props.item.state));
  const spelled = createMemo(() => spelledOf(props.item.install));
  const offer = createMemo(() => OFFER[offerOf(props.item)]);
  // An absent reason and a reason that is nothing are different
  // states, and only the first one leaves the control pressable.
  const because = (): Pick<ButtonProps, "why"> => {
    const word = offer().why;
    return word === null ? {} : { why: say(word) };
  };
  return (
    <li class="flex flex-col gap-tight border-t border-edge py-snug">
      <div class="flex flex-wrap items-center gap-snug">
        <span class="flex size-glyph shrink-0 items-center justify-center">
          <span class={`inline-block size-dot rounded-pill ${dotOf(props.item)}`} />
        </span>
        <Show
          when={props.item.homepage}
          fallback={<span class="font-mono text-label text-text">{props.item.name}</span>}
        >
          {(site) => (
            <Tip text={site()}>
              {(hint) => (
                <a
                  class="font-mono text-label text-text underline decoration-g3 underline-offset-2 hover:decoration-accent"
                  href={site()}
                  target="_blank"
                  rel="noreferrer"
                  aria-describedby={hint}
                >
                  {props.item.name}
                </a>
              )}
            </Tip>
          )}
        </Show>
        <span class="text-note text-text-faint">{say(stateKey(props.item.state))}</span>
        <Show when={props.item.need === "optional"}>
          <span class="text-note text-text-disabled">{say("machine_optional")}</span>
        </Show>
        <Show when={version()}>
          {(said) => <span class="min-w-0 truncate text-note text-text-quiet">{said()}</span>}
        </Show>
        <span class="flex-1" />
        <Show
          when={spelled()}
          fallback={<span class="text-note text-text-disabled">{say("machine_no_recipe")}</span>}
        >
          {(how) => (
            <div class="flex shrink-0 items-center gap-tight">
              <Button
                label={say("machine_install")}
                tone={offer().tone}
                {...because()}
                onPress={() => {
                  props.onInstall(props.item.name);
                }}
              />
              <Button
                label={say("setup_copy")}
                tone="quiet"
                onPress={() => {
                  void navigator.clipboard.writeText(how());
                }}
              />
            </div>
          )}
        </Show>
      </div>
      <p class="text-note text-text-faint">{props.item.enables}</p>
      <Show when={spelled()}>
        {(how) => (
          <code class="block whitespace-pre-wrap wrap-anywhere rounded-control bg-chrome px-snug py-tight font-mono text-note text-text-quiet">
            {how()}
          </code>
        )}
      </Show>
    </li>
  );
}

function Column(props: {
  readonly title: string;
  // What the city still names as missing for this column's tier, and
  // `null` when it has answered about no such tier.
  readonly missing: readonly string[] | null;
  readonly items: readonly DoctorItem[];
  readonly onInstall: (item: string) => void;
}) {
  const say = useSay();
  const far = createMemo(() => standing(props.items, props.missing?.length ?? null));
  const named = createMemo<readonly string[]>(() => props.missing ?? []);
  return (
    <section class="flex min-w-0 flex-col gap-snug" aria-label={props.title}>
      <h2 class="text-heading font-heading text-text">{props.title}</h2>
      <Progress label={say("machine_progress_label")} done={far().done} total={far().total} />
      {/* The city's own verdict on this tier, which is not the rows
          below it: a run of interchangeable items collapses into the
          one thing a person is still missing, so a column of empty
          rows can be a column with nothing left to do. The names are
          set apart by the gap between them rather than joined by a
          word, because a conjunction would be a second authority on
          how a list reads in each language. */}
      <Show when={named().length > 0}>
        <p class="flex flex-wrap items-baseline gap-tight text-note">
          <span class="text-text-quiet">{say("machine_missing")}</span>
          <For each={named()}>{(name) => <span class="font-mono text-alert">{name}</span>}</For>
        </p>
      </Show>
      <ul class="flex flex-col">
        <For each={props.items}>
          {(each) => <Row item={each} onInstall={props.onInstall} />}
        </For>
      </ul>
    </section>
  );
}

// One answer, drawn. Separate from the ask so the gallery can show
// this machine in states nobody's own machine happens to be in.
export function MachineReport(props: {
  readonly answer: DoctorAnswer;
  readonly onInstall?: (item: string) => void;
}) {
  const say = useSay();
  const install = (item: string) => {
    props.onInstall?.(item);
  };
  return (
    <div class="grid min-w-0 grid-cols-1 gap-wide @lg/page:grid-cols-2">
      <Column
        title={say("machine_required")}
        items={required(props.answer)}
        missing={outstanding(props.answer, "use")}
        onInstall={install}
      />
      <Column
        title={say("machine_recommended")}
        items={recommended(props.answer)}
        missing={outstanding(props.answer, "develop")}
        onInstall={install}
      />
    </div>
  );
}

// Twelve rows in the shape the real ones take, while the city is being
// asked. Exported so the gallery can hold this state still.
export function MachineSkeleton() {
  const say = useSay();
  return (
    <ul class="flex flex-col" aria-label={say("machine_checking")}>
      <For each={[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]}>
        {() => (
          <li class="flex items-center gap-snug border-t border-edge py-snug">
            <span class="inline-block size-dot animate-pulse rounded-pill bg-mark" />
            <span class="h-tight w-tree animate-pulse rounded-pill bg-raised" />
          </li>
        )}
      </For>
    </ul>
  );
}

// The city has not answered about this machine. Not a grey word: a
// state with the one action that leaves it.
export function MachineUnchecked(props: { readonly onRecheck: () => void }) {
  const say = useSay();
  return (
    <div class="flex flex-wrap items-center gap-snug">
      <span class="text-note text-text-faint">{say("machine_unasked")}</span>
      <Button
        label={say("machine_recheck")}
        tone="secondary"
        onPress={() => {
          props.onRecheck();
        }}
      />
    </div>
  );
}

// Which release this city is, and - only if asked - which one npm has.
//
// **The press is the whole trigger.** `ask` is called from the handler
// rather than from the component body, where Solid gives it no reactive
// owner: the answer is therefore watched by nobody, so neither a
// reconnect nor an event re-asks it, and opening this page costs no
// request. A city that polled a registry would be spending the promise
// `QUICKSTART.md` opens with on a question nobody asked.
//
// Nothing here updates anything. The two commands are printed for a
// person to run, because `sprawling install` owns the archive path and
// npm owns its own, and a third party writing over either would be a
// second authority for where this binary lives.
function Release() {
  const ui = useUi();
  const say = useSay();
  const [held, setHeld] = createSignal<Accessor<Answer | undefined>>();
  const [asking, setAsking] = createSignal(false);
  const answer = createMemo(() => {
    const now = held()?.();
    return now !== undefined && "release" in now ? now.release : undefined;
  });
  // One memo per state, because the check and the read have to happen
  // on one value: asking `"stands" in answer()` and then reading
  // `answer().stands` are two calls, and the second is not narrowed by
  // the first.
  const refused = createMemo(() => {
    const found = answer();
    return found !== undefined && "refused" in found ? found.refused : undefined;
  });
  const unreleased = createMemo(() => {
    const found = answer();
    return found !== undefined && "unreleased" in found ? found.unreleased : undefined;
  });
  const stands = createMemo(() => {
    const found = answer();
    return found !== undefined && "stands" in found ? found.stands : undefined;
  });
  createEffect(() => {
    if (answer() !== undefined) {
      setAsking(false);
    }
  });
  const check = () => {
    setAsking(true);
    // First press opens the slot and sends; every press after it sends
    // again, because "check again" means now rather than what was
    // already answered.
    const slot = ui.conn.asking.ask(QUERIES.release);
    setHeld(() => slot);
    ui.conn.asking.refresh(QUERIES.release);
  };
  return (
    <section class="flex min-w-0 flex-col gap-snug rounded-control bg-chrome p-base">
      <div class="flex flex-wrap items-center gap-snug">
        <h2 class="grow text-label text-text">{say("release_title")}</h2>
        <Button label={say("release_check")} tone="secondary" loading={asking()} onPress={check} />
      </div>
      <p class="text-note text-text-soft">{say("release_never")}</p>
      <Show when={refused()}>
        {(found) => (
          <div class="flex flex-col gap-tight">
            <p class="text-note text-alert">{say("release_refused")}</p>
            <p class="text-note text-text-soft">{found().refusal.recovery}</p>
          </div>
        )}
      </Show>
      <Show when={unreleased()}>
        {(found) => (
          <div class="flex flex-col gap-tight">
            <p class="text-note text-text">{say("release_source")}</p>
            <p class="text-note text-text-soft">
              {say("release_newest", {
                version: found().newest.version,
                released: found().newest.released,
              })}
            </p>
          </div>
        )}
      </Show>
      <Show when={stands()}>
        {(found) => (
          <div class="flex flex-col gap-tight">
            <p class="text-note text-text">
              {say("release_mine", {
                version: found().mine.version,
                released: found().mine.released,
              })}
            </p>
            <Switch>
              <Match when={found().verdict === "current"}>
                <p class="text-note text-accent">{say("release_current")}</p>
              </Match>
              <Match when={found().verdict === "ahead"}>
                <p class="text-note text-text-soft">
                  {say("release_ahead", { version: found().newest.version })}
                </p>
              </Match>
              <Match when={found().verdict === "behind"}>
                <p class="text-note text-alert">
                  {say("release_behind", {
                    version: found().newest.version,
                    released: found().newest.released,
                  })}
                </p>
                <code class="rounded-control bg-raised px-base py-snug font-mono text-note text-text">
                  {UPDATE_NPM}
                </code>
                <p class="text-note text-text-soft">{say("release_archive")}</p>
                <a
                  class="text-note text-accent underline"
                  href={RELEASES}
                  target="_blank"
                  rel="noreferrer"
                >
                  {RELEASES}
                </a>
                <p class="text-note text-text-soft">{say("release_manual")}</p>
              </Match>
            </Switch>
          </div>
        )}
      </Show>
    </section>
  );
}

// This machine, item by item, with the one command that changes it.
export function Machine() {
  const ui = useUi();
  const say = useSay();
  const held = ui.conn.asking.ask(QUERIES.doctor);
  const answer = createMemo(() => {
    const now = held();
    return now !== undefined && "doctor" in now ? now.doctor : undefined;
  });
  const [asking, setAsking] = createSignal(false);
  // A fresh answer ends the wait, whoever asked for it.
  createEffect(() => {
    if (answer() !== undefined) {
      setAsking(false);
    }
  });
  // The city writes one log line when it has looked at this machine
  // again, and that line is how this page knows the probe is over. A
  // read sent on the same tick as the command would arrive first and
  // answer with the machine as it was before, because probing is a
  // dozen programs started and asked their version.
  //
  // A city running with its log off writes no such line. `check again`
  // is then the whole mechanism, which is why it stays a control the
  // person can press rather than something the page does for them.
  const looked = createMemo(
    () => ui.conn.belief.logs.filter((line) => line.module === "bin::doctor").length,
  );
  createEffect(
    on(
      looked,
      () => {
        ui.conn.asking.refresh(QUERIES.doctor);
      },
      { defer: true },
    ),
  );
  // Each of the two asks straight away as well, so the page is never
  // left waiting on a line that may not come; that first answer is
  // whatever the city holds now, and the log line brings the second.
  const recheck = () => {
    setAsking(true);
    ui.conn.command(doctorRefresh());
    ui.conn.asking.refresh(QUERIES.doctor);
  };
  const install = (item: string) => {
    setAsking(true);
    ui.conn.command(doctorInstall(item));
    ui.conn.asking.refresh(QUERIES.doctor);
  };
  const reaching = () => {
    const kind = ui.conn.state().kind;
    return kind === "opening" || kind === "handshaking" || kind === "live";
  };
  return (
    <div class="flex min-w-0 flex-col gap-wide">
      <Release />
      <div class="flex flex-wrap items-center gap-snug">
        <Button
          label={say("machine_recheck")}
          tone="secondary"
          loading={asking()}
          onPress={recheck}
        />
        <code class="rounded-control bg-chrome px-base py-snug font-mono text-note text-text">{DOCTOR}</code>
        <Button
          label={say("setup_copy")}
          tone="quiet"
          onPress={() => {
            void navigator.clipboard.writeText(DOCTOR);
          }}
        />
      </div>
      <Show
        when={answer()}
        fallback={
          <Show when={reaching()} fallback={<MachineUnchecked onRecheck={recheck} />}>
            <MachineSkeleton />
          </Show>
        }
      >
        {(found) => <MachineReport answer={found()} onInstall={install} />}
      </Show>
    </div>
  );
}
