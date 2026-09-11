// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The box a person writes in. One textarea, Enter sends, and beside it
// the three things that belong to a message - which model answers,
// which room hears it, and how hard the model thinks - behind one
// control, plus the way to stop a run while it is going. Nothing here
// decides where a message goes; the page does.
//
// A line that begins with `/` is a command rather than a message, and
// the menu that opens over the box is the same list as the selector's,
// reading the same table the Ctrl-K palette reads (`core/slash.ts`).

import { Show, createMemo, createSignal, untrack } from "solid-js";

import type { RunBelief, Sending } from "../../core/belief";
import type { Key } from "../../core/lang";
import { EFFORTS } from "../../core/prefs";
import { MAYOR, current } from "../../core/route";
import { completed, find, offered, parse } from "../../core/slash";
import type { Reached, Slash, SlashHands } from "../../core/slash";
import { selectModel } from "../../core/commands";
import { canRecord, record, type Heard, type Recording } from "../../core/speaking";
import { Address } from "../../core/address";
import { useCommand, useGo, useSay, useUi } from "../../ui";
import { Popover } from "../parts/popover";
import type { PopoverColumn, PopoverItem } from "../parts/popover";
import { Option } from "effect";

// What the send control is spelled, for each of the three places a
// message can land. `queued` is the one a streaming page would otherwise
// hide: the run is inside a tool call, and the words wait for a boundary.
const SPELLING: Record<Sending, Key> = {
  dispatch: "talk_send",
  steer: "talk_steer",
  queued: "talk_steer_queued",
};

// The page-level motion budget: the box falling from the middle of an
// empty room to the foot of a full one (client-SPEC 4-11, §13.5).
const DROP_MS = 400;
const EASING = "cubic-bezier(0.2, 0, 0, 1)";

// Which list is over the box, if any. One signal rather than two, so
// the two can never both be open.
type Opened = "none" | "commands" | "choice";

// The three columns of the combined selector, named where both the
// builder and the applier read them.
const MODEL = "model";
const WORKSPACE = "workspace";
const EFFORT = "effort";
const COMMANDS = "commands";

// A run as a verb reaches it, or nothing when there is no run.
function reached(run: RunBelief | undefined): Reached | null {
  return run === undefined ? null : { run: run.run, at: run.lastSeq };
}

export interface ComposerProps {
  readonly placeholder: string;
  readonly sending: Sending;
  // Where the unsent words are kept, when they are kept at all: the
  // room or the run this box speaks to.
  readonly draft?: string | undefined;
  // Both answer whether the frame went out, so the box can keep the
  // words when it did not.
  readonly onSend: (text: string) => boolean;
  readonly onStop: () => boolean;
  // Whether this city has an endpoint that transcribes. A microphone
  // on a city with none is a button whose only answer is a refusal.
  readonly hearing?: boolean;
}

export function Composer(props: ComposerProps) {
  const ui = useUi();
  const say = useSay();
  const go = useGo();
  const command = useCommand();
  const [text, setText] = createSignal(untrack(() => (props.draft === undefined ? "" : ui.prefs.draft(props.draft))));
  const [kept, setKept] = createSignal(false);
  const [box, setBox] = createSignal<HTMLTextAreaElement>();
  const [form, setForm] = createSignal<HTMLFormElement>();
  const [open, setOpen] = createSignal<Opened>("none");
  const keep = (words: string) => {
    if (props.draft !== undefined) ui.prefs.setDraft(props.draft, words);
  };

  const grow = () => {
    const area = box();
    if (area === undefined) return;
    area.style.height = "auto";
    area.style.height = `${String(Math.min(area.scrollHeight, 240))}px`;
  };
  // Whatever put words in the box - a transcription, a completion, a
  // command that empties it - goes through here, so the draft store and
  // the box's height never disagree with what is on screen.
  const write = (words: string) => {
    setText(words);
    keep(words);
    requestAnimationFrame(grow);
  };

  // The box falls from the middle of an empty room to the foot of a
  // full one the first time somebody sends. Where it lands is the
  // page's decision, so the drop is measured rather than declared: the
  // box remembers where it was, lets the page relay out, and rides the
  // difference back to zero on the compositor.
  const drop = () => {
    const held = form();
    if (held === undefined) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    const was = held.getBoundingClientRect().top;
    requestAnimationFrame(() => {
      const now = held.getBoundingClientRect().top;
      const moved = was - now;
      if (Math.abs(moved) < 1) return;
      held.animate(
        [{ transform: `translateY(${String(moved)}px)` }, { transform: "translateY(0)" }],
        { duration: DROP_MS, easing: EASING },
      );
    });
  };

  const submit = () => {
    const words = text().trim();
    if (words === "") return;
    if (props.onSend(words)) {
      write("");
      setKept(false);
      drop();
    } else {
      setKept(true);
    }
  };

  // -------------------------------------------------- what the city offers

  const endpoints = ui.conn.asking.ask("endpoint_view");
  const answer = createMemo(() => {
    const held = endpoints();
    return held !== undefined && "endpoints" in held ? held.endpoints : undefined;
  });
  const models = createMemo(() =>
    (answer()?.endpoints ?? []).flatMap((endpoint) =>
      endpoint.models.map((model) => ({ endpoint: endpoint.name, model })),
    ),
  );
  const main = createMemo(() => answer()?.chosen.find((each) => each.tag === "main"));

  // The room this box speaks to, read off the address bar: the page
  // that mounted a composer is the page that named the room, and a
  // second copy of that name here could only disagree with it.
  const here = createMemo(() => {
    const view = Option.getOrNull(current(ui.bar));
    return view !== null && view.kind === "talk" ? view.address : null;
  });
  // Every room a person could move this conversation to: the buildings
  // the city knows, and the rooms runs have already opened.
  const city = ui.conn.asking.ask("city_view");
  const rooms = createMemo(() => {
    const held = city();
    const named = new Set<string>([MAYOR]);
    if (held !== undefined && "city" in held) {
      for (const building of held.city.buildings) named.add(building.addr);
    }
    for (const run of Object.values(ui.conn.belief.runs)) {
      if (run.addr !== null) named.add(run.addr);
    }
    return [...named].sort((a, b) => a.localeCompare(b));
  });
  // The run this box would steer. The same rule the page uses, asked
  // here because a command typed as `/stop` must reach the run the stop
  // button beside it reaches.
  const live = createMemo<RunBelief | undefined>(() => {
    const room = here();
    if (room === null) return undefined;
    return Object.values(ui.conn.belief.runs)
      .filter((run) => run.addr === room && run.doing.kind !== "frozen")
      .sort((a, b) => (b.started ?? 0) - (a.started ?? 0))
      .at(0);
  });

  const hands = (): SlashHands => ({
    command,
    go,
    here: here(),
    live: reached(live()),
    newest: (room) => {
      return reached(
        Object.values(ui.conn.belief.runs)
          .filter((run) => run.addr === room)
          .sort((a, b) => (b.started ?? 0) - (a.started ?? 0))
          .at(0),
      );
    },
    models: models(),
    effort: ui.prefs.effort(),
    setEffort: (effort) => {
      ui.prefs.setEffort(effort);
    },
    goal: say("talk_goal"),
    write,
  });

  // -------------------------------------------------------- the two lists

  const showing = createMemo(() => (open() === "commands" ? offered(text()) : []));
  // Which list is drawn, as one value a keyed `Show` can remount on: a
  // menu that borrows the keyboard and a selector that takes it are two
  // popovers, not one popover told twice.
  const mode = createMemo<Opened | undefined>(() => {
    if (open() === "choice") return "choice";
    return open() === "commands" && showing().length > 0 ? "commands" : undefined;
  });
  const columns = createMemo<readonly PopoverColumn[]>(() => {
    if (open() === "commands") {
      return [
        {
          id: COMMANDS,
          label: say("talk_commands"),
          items: showing().map((each) => ({
            id: each.spelling,
            label: each.spelling,
            hint: each.grammar === "" ? say(each.about) : `${each.grammar} · ${say(each.about)}`,
          })),
        },
      ];
    }
    const chosen = main();
    return [
      {
        id: MODEL,
        label: say("talk_column_model"),
        items: models().map((each) => ({
          id: `${each.endpoint}\u0000${each.model}`,
          label: each.model,
          hint: each.endpoint,
          chosen: chosen?.model === each.model && chosen.endpoint === each.endpoint,
        })),
      },
      {
        id: WORKSPACE,
        label: say("talk_column_workspace"),
        items: rooms().map((room) => ({ id: room, label: room, chosen: room === here() })),
      },
      {
        id: EFFORT,
        label: say("talk_column_effort"),
        items: EFFORTS.map((effort) => ({
          id: effort,
          label: say(`effort_${effort}`),
          chosen: effort === ui.prefs.effort(),
        })),
      },
    ];
  });

  // A verb the menu offered, once. A verb that still needs its
  // argument is completed into the box rather than run on nothing.
  const pick = (chosen: Slash) => {
    const call = parse(text());
    const needs = chosen.grammar.startsWith("<");
    const typed = call !== null && call.verb === chosen.spelling;
    if (typed && (!needs || call.rest !== "")) {
      chosen.run(hands(), call);
      setOpen("none");
      return;
    }
    write(`${chosen.spelling} `);
    box()?.focus();
  };

  // What a row does when it is applied. The command list runs its verb;
  // the three columns each write one preference and leave the list open
  // so a person can set the next one.
  const applied = (column: PopoverColumn, item: PopoverItem) => {
    if (column.id === COMMANDS) {
      const chosen = find(item.id);
      if (chosen !== undefined) pick(chosen);
      return;
    }
    if (column.id === MODEL) {
      const [endpoint, model] = item.id.split("\u0000");
      if (endpoint !== undefined && model !== undefined && model !== "") {
        command(selectModel(endpoint, model, "main"));
      }
      return;
    }
    if (column.id === WORKSPACE) {
      const address = Option.getOrNull(Address.option(item.id));
      if (address !== null) {
        go({ kind: "talk", address });
        setOpen("none");
      }
      return;
    }
    const level = EFFORTS.find((known) => known === item.id);
    if (level !== undefined) ui.prefs.setEffort(level);
  };

  let menuKeys: ((event: KeyboardEvent) => boolean) | null = null;

  // ------------------------------------------------------------- speaking

  const [taking, setTaking] = createSignal<Recording | null>(null);
  const [hearing, setHearing] = createSignal(false);
  const [refused, setRefused] = createSignal(false);
  // The words land in the box rather than being sent: what somebody
  // said is a draft like any other, and a machine that heard it wrongly
  // must be correctable before it costs a run.
  const speak = () => {
    const going = taking();
    if (going === null) {
      setRefused(false);
      void record(ui.origin).then((started) => {
        setTaking(() => started);
        setRefused(started === null);
      });
      return;
    }
    setTaking(null);
    setHearing(true);
    // The words are appended to whatever is in the box at the moment
    // the city answers, which is why the read happens inside the
    // updater rather than beside it: somebody goes on typing while a
    // recording is being transcribed.
    const settle = (answer: Heard) => {
      setHearing(false);
      setRefused(answer.kind === "refused");
      if (answer.kind !== "text") {
        return;
      }
      const before = untrack(text);
      write(before === "" ? answer.text : `${before} ${answer.text}`);
    };
    void going.stop().then(settle);
  };

  return (
    <form
      ref={setForm}
      class="relative rounded-panel border border-dashed border-g4 bg-g1 p-base shadow-composer focus-within:border-solid focus-within:border-accent"
      onSubmit={(event) => {
        event.preventDefault();
        submit();
      }}
      aria-label={say("region_composer")}
    >
      <Show keyed when={mode()}>
        {(held) => (
          <Popover
            label={held === "commands" ? say("talk_commands") : say("talk_choose")}
            columns={columns()}
            onApply={applied}
            onClose={() => {
              setOpen("none");
              box()?.focus();
            }}
            bind={
              held === "commands"
                ? (keys) => {
                    menuKeys = keys;
                  }
                : undefined
            }
          />
        )}
      </Show>
      <textarea
        ref={(area) => {
          setBox(area);
          // A draft restored on mount is taller than one row.
          requestAnimationFrame(grow);
        }}
        class="block w-full resize-none bg-transparent text-body leading-relaxed text-text outline-none placeholder:text-text-disabled"
        rows={1}
        placeholder={props.placeholder}
        // A placeholder is not a name: it is gone as soon as somebody
        // types, and this is the control the page exists for. Same
        // words, so the two never disagree.
        aria-label={props.placeholder}
        value={text()}
        autofocus
        onInput={(event) => {
          const words = event.currentTarget.value;
          setText(words);
          keep(words);
          grow();
          setOpen(words.startsWith("/") ? "commands" : "none");
        }}
        onKeyDown={(event) => {
          if (open() === "commands" && showing().length > 0) {
            if (event.key === "Tab") {
              event.preventDefault();
              write(completed(text()));
              return;
            }
            if (menuKeys?.(event) === true) {
              event.preventDefault();
              return;
            }
          }
          if (event.key === "Enter" && !event.shiftKey && !event.isComposing) {
            event.preventDefault();
            submit();
          }
        }}
      />
      <div class="mt-snug flex items-center justify-between gap-base text-note text-text-faint">
        <div class="flex min-w-0 items-center gap-base">
          <button
            type="button"
            class="flex min-w-0 items-center gap-tight rounded-pill bg-g2 px-base py-tight text-note text-text-quiet hover:bg-g3"
            onClick={() => {
              setOpen((held) => (held === "choice" ? "none" : "choice"));
            }}
            aria-expanded={open() === "choice"}
            title={say("talk_choose")}
          >
            <span class="min-w-0 truncate">{main()?.model ?? say("talk_no_model")}</span>
            <Show when={here()}>
              {(room) => <span class="min-w-0 truncate text-text-disabled"> · {room()}</span>}
            </Show>
            <span class="shrink-0 text-text-disabled">
              {" · "}
              {say("talk_effort")} {say(`effort_${ui.prefs.effort()}`)}
            </span>
          </button>
          <Show when={props.hearing === true && canRecord()}>
            <button
              type="button"
              class={`rounded-pill px-base py-tight text-note ${taking() === null ? "bg-g2 text-text-quiet hover:bg-g3" : "bg-alert text-g0"}`}
              disabled={hearing()}
              onClick={speak}
            >
              {hearing() ? say("talk_hearing") : taking() === null ? say("talk_record") : say("talk_recording")}
            </button>
          </Show>
          <Show when={kept()}>
            <span class="text-alert">{say("talk_not_live")}</span>
          </Show>
          <Show when={refused()}>
            <span class="text-alert">{say("link_refused")}</span>
          </Show>
        </div>
        <div class="flex shrink-0 items-center gap-base">
          <Show when={props.sending !== "dispatch"}>
            <button
              type="button"
              class="rounded-control px-snug py-tight text-label text-text-quiet hover:bg-g2 hover:text-alert"
              onClick={() => props.onStop()}
            >
              {say("talk_stop")}
            </button>
          </Show>
          <button
            type="submit"
            class="rounded-control bg-accent px-base py-tight text-label text-g0 hover:bg-accent-hover disabled:bg-g3 disabled:text-text-disabled"
            disabled={text().trim() === ""}
          >
            {say(SPELLING[props.sending])}
          </button>
        </div>
      </div>
    </form>
  );
}
