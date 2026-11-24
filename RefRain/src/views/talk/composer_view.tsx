// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { Option } from "effect";
import { createEffect, createMemo, createSignal, onCleanup, onSettled, untrack } from "solid-js";
import { QUERIES } from "../../core/asking";
import { heldIn } from "../../core/belief/rooms";
import type { Sending } from "../../core/doing";
import { runInFront } from "../../core/in_front";
import { say } from "../../core/lang";
import { current } from "../../core/route";
import { find, parse } from "../../core/slash";
import type { Slash } from "../../core/slash_hands";
import { storeValue } from "../../core/solid_store";
import { readable } from "../../core/store";
import { markEndAtFrame, markStart } from "../../core/timing";
import { ui } from "../../ui";
import type { Address, RunId } from "../../wire";
import type { PopoverBinding, PopoverColumn, PopoverRow } from "../parts/popover";
import { attachmentsAt, attachmentStorageAt, removeAttachments } from "./attachments";
import { TextBox } from "./box";
import { faceOf } from "./coin_face";
import {
  boxKey,
  completedAt,
  configLevel,
  menuColumns,
  offeredBy,
  pickSlash,
  picksFor,
  sessionModel,
  slashHands,
  startedIn,
  type BoxKey,
} from "./composer";
import { ComposerCommands, ComposerControls, ComposerFooter } from "./composer_controls";
import { ComposerFrame, type ComposerFrameProps } from "./composer_frame";
import { draftAt } from "./draft";
import { DropZone } from "./drop_zone";
import { insertAt } from "./dropping";
import { hand, IDLE, settle, type Handing } from "./handing";
import { hearQuotes, joined } from "./quoting";
import { sessionHands } from "./session_hands";
import {
  recheckSpeech,
  speechAt,
  submitSpeech,
  watchSpeech,
  type SpeechResult,
  type SpeechTarget,
} from "./speech";
import { keepSelection, recalled, restoreSelection } from "./standing";
import TypedLine from "./typed_line_view";

export interface ComposerProps {
  readonly placeholder: string;
  readonly started?: boolean | undefined;
  readonly sending: Sending;
  readonly draft?: string | undefined;
  readonly onSend?: ((text: string) => boolean) | undefined;
  readonly onStop: () => boolean;
  readonly hearing?: boolean | undefined;
  readonly band?: (() => JSX.Element) | undefined;
  readonly room?: Address | undefined;
  readonly door?: (() => JSX.Element) | undefined;
  readonly conversation?: SpeechTarget | undefined;
  readonly contextRun?: RunId | undefined;
  readonly onDispatch?: ((task: string) => void) | undefined;
}

export default function Composer(props: ComposerProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const effort = storeValue(u.effort);
  const belief = storeValue(u.conn.belief);
  const policy = storeValue(u.policy);
  const attached = storeValue(() => attachmentsAt(props.draft));
  const attachmentStorage = storeValue(() => attachmentStorageAt(props.draft));
  const speech = storeValue(() => speechAt(props.draft));
  const unkept = storeValue(() =>
    props.draft === undefined ? readable(false) : u.prefs.draftUnkept(props.draft),
  );
  const [sendProblem, setSendProblem] = createSignal<SpeechResult | null>(null);
  let place = untrack(() => props.draft);
  let keptDraft = draftAt(u.prefs, place);
  const [text, setText] = createSignal(keptDraft.read);
  const [kept, setKept] = createSignal(false);
  const [sent, setSent] = createSignal(0);
  const [focused, setFocused] = createSignal(false);
  const [open, setOpen] = createSignal(false);
  const [activeId, setActiveId] = createSignal<string | null>(null);
  const [pointed, setPointed] = createSignal<string>();
  const [menuBinding, setMenuBinding] = createSignal<PopoverBinding | null>(null);
  const field = new TextBox(text);
  const fieldNode = storeValue(field.node);
  let handing: Handing = IDLE;

  function arrived(): void {
    field.grow();
    restoreSelection(place, field.element);
  }

  function moveTo(next: string | undefined): void {
    if (next === place) return;
    keptDraft.flush();
    keepSelection(place, field.element);
    place = next;
    keptDraft = draftAt(u.prefs, next);
    const words = keptDraft.read;
    setText(words);
    setSendProblem(null);
    setOpen(words.startsWith("/"));
    requestAnimationFrame(arrived);
  }

  function write(words: string): void {
    setText(words);
    setOpen(words.startsWith("/"));
    keptDraft.replace(words);
    requestAnimationFrame(() => {
      field.grow();
    });
  }

  function quoted(quote: string): void {
    keepSelection(place, field.element);
    write(joined(text(), quote));
    requestAnimationFrame(arrived);
  }

  createEffect(() => props.draft, moveTo);
  createEffect(
    () => text(),
    (words) => {
      field.sync(words);
    },
  );
  createEffect(
    () => props.draft,
    (draft) => (draft === undefined ? undefined : hearQuotes(draft, quoted)),
  );
  watchSpeech(
    u.conn,
    () => (props.conversation === undefined ? undefined : props.draft),
    speech,
    (draft, accepted) => {
      moveTo(draft);
      if (text().trim() === accepted) write("");
    },
  );
  createEffect(belief, (now) => {
    const next = settle(handing, now, text());
    handing = next.handing;
    if (next.box !== null) write(next.box);
  });
  onSettled(() => {
    const frame = requestAnimationFrame(arrived);
    const saveLeaving = (): void => {
      keptDraft.flush();
    };
    window.addEventListener("pagehide", saveLeaving);
    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener("pagehide", saveLeaving);
    };
  });
  onCleanup(() => {
    keepSelection(place, field.element);
    keptDraft.flush();
  });

  const endpoints = storeValue(u.conn.asking.ask(QUERIES.endpoints));
  const city = storeValue(u.conn.asking.ask(QUERIES.city));
  const buildings = createMemo(() => {
    const response = city();
    return response !== undefined && "city" in response ? response.city.buildings : undefined;
  });
  const answer = createMemo(() => {
    const response = endpoints();
    return response !== undefined && "endpoints" in response ? response.endpoints : undefined;
  });
  const shown = createMemo(() => {
    if (props.room === undefined) return Option.getOrNull(current(u.bar));
    const target = props.conversation;
    return {
      kind: "talk" as const,
      address: props.room,
      ...(target?.historical === true && target.session !== null
        ? { session: target.session }
        : {}),
    };
  });
  const here = createMemo(() => {
    const view = shown();
    return view?.kind === "talk" ? view.address : null;
  });
  const live = createMemo(() => {
    const view = shown();
    return view === null ? undefined : runInFront(belief(), view);
  });
  const startedHere = createMemo(() => {
    const room = here();
    return room !== null && startedIn(heldIn(belief(), room), belief().sessions[room]);
  });
  const model = createMemo(() => {
    const room = here();
    return room === null
      ? null
      : sessionModel(heldIn(belief(), room), belief().sessions[room] ?? null);
  });
  const roomConfig = storeValue(() => {
    const room = here();
    return room === null ? null : u.conn.asking.ask({ config: { addr: room } });
  });
  const offered = createMemo(() =>
    offeredBy(answer(), effort() ?? configLevel(roomConfig()), picksFor(u, model())),
  );
  const showing = createMemo(() => (open() ? menuColumns(lang(), text(), offered().argued) : []));

  function closeMenu(): void {
    setOpen(false);
    field.element?.focus();
  }

  function pick(chosen: Slash): void {
    const view = shown();
    const rest = pickSlash(
      chosen,
      text(),
      slashHands({
        command: u.send,
        go: u.go,
        here: here(),
        live: live(),
        belief: belief(),
        buildings: buildings(),
        models: offered().models,
        effort: effort(),
        setEffort: u.chooseEffort,
        policy: policy(),
        setPolicy: u.choosePolicy,
        goal: say(lang(), "talk_goal"),
        write,
        ...sessionHands(u, here(), view?.kind === "talk" ? view.session : undefined),
      }),
    );
    if (rest === null) return;
    write(rest);
    field.element?.focus();
  }

  function submit(): void {
    const words = text().trim();
    if (words === "") return;
    const call = parse(words);
    const chosen = call === null ? undefined : find(call.verb);
    if (chosen !== undefined) {
      pick(chosen);
      return;
    }
    markStart("send_shown");
    if (props.conversation !== undefined) {
      const origin = props.draft;
      void submitSpeech(u.conn, props.conversation, words, {
        goal: say(lang(), "talk_goal"),
        effort: effort(),
        policy: policy(),
        onDispatch: props.onDispatch,
      }).then((result) => {
        if (origin !== props.draft) return;
        setSendProblem(result === "sent" || result === "busy" ? null : result);
        if (result === "sent") markEndAtFrame("send_shown");
      });
      return;
    }
    if (props.onSend?.(words) !== true) {
      setKept(true);
      return;
    }
    markEndAtFrame("send_shown");
    handing = hand(words, belief());
    write("");
    setKept(false);
    setSent((count) => count + 1);
  }

  function act(key: BoxKey, event: KeyboardEvent): void {
    switch (key) {
      case "complete":
        event.preventDefault();
        write(completedAt(text(), pointed()));
        return;
      case "recall": {
        const room = here();
        const last = room === null ? null : recalled(belief(), room);
        if (last === null) return;
        event.preventDefault();
        write(last);
        return;
      }
      case "send":
        event.preventDefault();
        submit();
        return;
      case "menu":
      case "type":
        return;
    }
  }

  function onKeydown(event: KeyboardEvent): void {
    const state = { menu: open() && showing().length > 0, empty: text() === "" };
    const first = boxKey(event, state);
    if (first === "menu" && menuBinding()?.keys(event) === true) {
      event.preventDefault();
      return;
    }
    act(first === "menu" ? boxKey(event, { ...state, menu: false }) : first, event);
  }

  function onInput(event: Event & { readonly currentTarget: HTMLTextAreaElement }): void {
    const words = event.currentTarget.value;
    setText(words);
    keptDraft.keep(words);
    field.grow();
    setOpen(words.startsWith("/"));
  }

  function onApply(_column: PopoverColumn, row: PopoverRow): void {
    const call = parse(row.id);
    const chosen = call === null ? undefined : find(call.verb);
    if (chosen === undefined) return;
    if (row.id !== chosen.spelling) write(row.id);
    pick(chosen);
  }

  const zone = new DropZone(u, (paths) => {
    write(insertAt(text(), field.element?.selectionStart ?? text().length, paths));
    field.element?.focus();
  });
  const zoneState = storeValue(zone.state);
  const hasDoor = createMemo(() => text() === "" && props.door !== undefined);
  const look = createMemo<ComposerFrameProps["look"]>(() => ({
    form: {
      "aria-label": say(lang(), "region_composer"),
      onsubmit: (event: SubmitEvent) => {
        event.preventDefault();
        submit();
      },
      ondragover: zone.enter,
      ondragleave: zone.leave,
      ondrop: zone.drop,
    },
    over: zoneState().over,
    box: {
      rows: 1,
      placeholder: hasDoor() ? undefined : props.placeholder,
      "aria-label": props.placeholder,
      "aria-activedescendant": showing().length > 0 ? activeId() : null,
      "aria-controls": showing().length > 0 ? menuBinding()?.controls.join(" ") : undefined,
      autofocus: true,
      onkeydown: onKeydown,
      oninput: onInput,
      onfocus: () => {
        setFocused(true);
      },
      onblur: () => {
        setFocused(false);
      },
    },
    attachBox: field.hold,
    attachments: {
      attachments: attached(),
      storageFailure: attachmentStorage(),
      lang: lang(),
      remove: (id) => {
        if (props.draft !== undefined) removeAttachments(props.draft, [id]);
      },
    },
    speech: {
      state: speech(),
      problem: sendProblem(),
      lang: lang(),
      recheck: () => {
        if (props.draft !== undefined) recheckSpeech(u.conn, props.draft);
      },
      onDraft:
        props.draft === undefined
          ? undefined
          : (diagnostic) => {
              quoted(`\n\`\`\`toml\n${diagnostic}\n\`\`\``);
              field.element?.focus();
            },
    },
  }));

  return (
    <ComposerFrame
      look={look()}
      door={hasDoor()}
      slots={{
        get menu() {
          return (
            <ComposerCommands
              open={open()}
              columns={showing()}
              onApply={onApply}
              onClose={closeMenu}
              bind={setMenuBinding}
              onCursor={setActiveId}
              onRow={setPointed}
            />
          );
        },
        get band() {
          return props.band?.();
        },
        get door() {
          return props.door?.();
        },
        get keys() {
          return (
            <ComposerControls
              room={here()}
              run={props.contextRun ?? props.conversation?.run}
              historical={props.conversation?.historical === true}
              face={faceOf(
                text(),
                props.conversation === undefined
                  ? live() !== undefined
                  : props.conversation.run !== undefined,
              )}
              sending={props.sending}
              delivery={speech().kind}
              onWords={
                props.hearing === true
                  ? (words) => write(text() === "" ? words : `${text()} ${words}`)
                  : undefined
              }
              onStop={() => {
                props.onStop();
                field.element?.focus();
              }}
            />
          );
        },
        get line() {
          return <TypedLine text={text()} box={fieldNode()} lit={focused() || text() !== ""} />;
        },
        get under() {
          return (
            <ComposerFooter
              room={here()}
              sent={sent()}
              words={text}
              unkept={unkept()}
              refused={zoneState().refused}
              picker={offered().picker}
              kept={kept()}
              draws={
                props.band !== undefined || props.started === true || startedHere()
                  ? "notice"
                  : "everything"
              }
            />
          );
        },
      }}
    />
  );
}
