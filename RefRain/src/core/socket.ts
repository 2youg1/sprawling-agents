// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import { writable } from "svelte/store";
import type { Readable } from "svelte/store";

import type { Lang } from "./lang";
import { createAsking } from "./asking";
import type { Asking } from "./asking";
import { createBelief } from "./belief";
import { createWatching } from "./watching";
import type { Watching } from "./watching";
import type { Belief } from "./belief";
import { NO_TAIL, appended } from "./live_output";
import type { Tail } from "./live_output";
import { decodeFrame, encodeFrame } from "./frames";
import { createUnsent, isSpeech } from "./unsent";
import { langOf, say } from "./lang";
import { createGapWalk } from "./gap_walk";
import {
  advance,
  connect as start,
  isLive,
  isStopped,
  newLink,
  unreadableRecord,
  unsentCommand,
} from "./link";
import { closeCity } from "./access";
import type { Link, LinkAction, LinkEvent, LinkState } from "./link";
import type { Dial, Line } from "./line";
import { AskId } from "../wire";
import type { CloseMode, Command, Query, RunId, ServerFrame } from "../wire";

export interface Connection {
  readonly state: Readable<LinkState>;
  readonly belief: Readable<Belief>;
  readonly asking: Asking;
  
  
  readonly live: Readable<Readonly<Partial<Record<RunId, Tail>>>>;
  
  readonly unsent: Readable<number>;
  
  
  readonly command: (command: Command) => boolean;
  readonly retry: () => void;
  
  
  readonly closeCity: (mode: CloseMode) => boolean;
  
  
  readonly closing: Readable<CloseMode | null>;
  readonly dismissRefusal: () => void;
  
  readonly markNoticesSeen: () => void;
  readonly monitor: Pick<Watching, "samples" | "watch" | "watchSummary" | "setBeat">;
}


export function bearing(token: string | null): Readonly<Record<string, string>> {
  return token === null ? {} : { authorization: `Bearer ${token}` };
}

function hidden(): boolean {
  return typeof document !== "undefined" && document.visibilityState === "hidden";
}

export function openConnection(dial: Dial, lang: Lang): Connection {
  let link: Link = newLink(lang);
  let line: Line | null = null;
  
  
  const now = () => Date.now();
  const state = writable<LinkState>(link.state);
  const store = createBelief(now);
  const unsent = createUnsent();
  const live = writable<Readonly<Partial<Record<RunId, Tail>>>>({});
  const closing = writable<CloseMode | null>(null);

  const queue: ServerFrame[] = [];
  let scheduled = false;
  
  
  let lastAsk = 0;
  
  
  let reconnect: ReturnType<typeof setTimeout> | null = null;

  
  function sendAsk(query: Query): AskId | null {
    lastAsk = lastAsk >= 0xffff_ffff ? 1 : lastAsk + 1;
    const askId = AskId.make(lastAsk);
    return sendText(encodeFrame({ ask: { ask_id: askId, query } })) ? askId : null;
  }

  function sendText(text: string): boolean {
    return line?.send(text) ?? false;
  }
  const watching = createWatching(sendText);

  const asking = createAsking(
    (query: Query) => (isLive(link) ? sendAsk(query) : null),
    now,
    
    
    (phrase, error) => {
      const lang = langOf(document.documentElement.lang);
      store.refused({ ...error, recovery: say(lang, phrase) });
    },
  );
  const walk = createGapWalk(sendAsk, store, asking, lang);

  function perform(action: LinkAction): void {
    switch (action.kind) {
      case "nothing":
        return;
      case "open":
        open();
        return;
      case "send":
        sendText(encodeFrame(action.frame));
        return;
      case "welcomed":
        store.refused(null);
        store.named(action.welcome.city ?? null);
        walk.welcomed(action.welcome.epoch ?? null, action.welcome.resume_from ?? null);
        unsent.release((command) => sendText(encodeFrame({ command })));
        watching.reconnected();
        return;
      case "deliver": {
        
        
        const bad = walk.fold(action.event);
        if (action.event.kind === "tool_result") {
          live.update(({ [action.event.run]: _settled, ...rest }) => rest);
        }
        asking.invalidate(action.event);
        if (bad !== null) store.refused(unreadableRecord(lang, bad));
        return;
      }
      case "answered": {
        const { ask_id, as_of, outcome } = action.answered;
        if ("refusal" in outcome) store.refused(outcome.refusal);
        
        
        if (walk.answered(ask_id, outcome)) return;
        if ("answer" in outcome && "city" in outcome.answer) {
          store.adoptCity(outcome.answer.city);
        }
        asking.answered(ask_id, as_of, outcome);
        return;
      }
      case "saying":
        store.say(action.delta);
        return;
      case "logged":
        store.logged(action.line);
        return;
      case "writing": {
        const { piece } = action;
        live.update((tails) => ({
          ...tails,
          [piece.run]: appended(tails[piece.run] ?? NO_TAIL, piece),
        }));
        return;
      }
      case "lagged":
        walk.lagged(action.from, action.to);
        return;
      case "sampled":
        watching.sampled(action.sample);
        return;
      case "wait":
        reconnect = setTimeout(() => {
          reconnect = null;
          step({ kind: "wait_elapsed" });
        }, action.ms);
        return;
      case "report":
        store.refused(action.error);
        return;
      case "close":
        line?.close();
        return;
    }
  }

  function step(event: LinkEvent): void {
    const was = link;
    const [next, action] = advance(link, event);
    link = next;
    if (next.state !== was.state) {
      state.set(next.state);
    }
    perform(action);
    if (isStopped(link)) {
      
      
      if (reconnect !== null) {
        clearTimeout(reconnect);
        reconnect = null;
      }
      const open = line;
      line = null;
      open?.close();
    }
  }

  
  function drain(): void {
    scheduled = false;
    store.batch(() => {
      for (const frame of queue.splice(0, queue.length)) {
        step({ kind: "received", frame });
      }
    });
  }

  function open(): void {
    
    
    const mine = () => line === opened;
    const opened: Line = dial({
      opened: (credential) => {
        if (mine()) step({ kind: "opened", credential });
      },
      heard: (text) => {
        if (mine()) heard(text);
      },
      closed: () => {
        if (!mine()) return;
        line = null;
        step({ kind: "closed" });
      },
    });
    line = opened;
  }

  function heard(text: string): void {
    const frame = decodeFrame(text);
    if (frame === null) {
      
      
      step({ kind: "undecodable" });
      return;
    }
    
    
    if ("welcome" in frame || "refusal" in frame) {
      store.batch(() => {
        drain();
        step({ kind: "received", frame });
      });
      return;
    }
    queue.push(frame);
    if (!scheduled) {
      scheduled = true;
      
      
      if (hidden()) setTimeout(drain, 0);
      else requestAnimationFrame(drain);
    }
  }

  
  function visibilityChanged(): void {
    if (queue.length > 0) drain();
    if (hidden() || reconnect === null) return;
    step(unbook({ kind: "wait_elapsed" }));
  }

  
  function unbook(event: LinkEvent): LinkEvent {
    if (reconnect !== null) clearTimeout(reconnect);
    reconnect = null;
    return event;
  }
  if (typeof document !== "undefined")
    document.addEventListener("visibilitychange", visibilityChanged);

  const [begun, first] = start(link);
  link = begun;
  state.set(begun.state);
  perform(first);

  return {
    state,
    belief: store.belief,
    asking,
    unsent: unsent.count,
    live,
    command(command) {
      switch (link.state.kind) {
        case "live":
          return sendText(encodeFrame({ command }));
        
        
        case "idle":
        case "refused":
        case "closed":
          return false;
        case "opening":
        case "handshaking":
        case "backoff":
          if (!isSpeech(command)) {
            store.refused(unsentCommand(lang, Object.keys(command).join()));
            return false;
          }
          unsent.hold(command);
          return true;
      }
    },
    retry() {
      closing.set(null);
      step(unbook({ kind: "retry" }));
    },
    closeCity(mode) {
      if (!isLive(link) || !sendText(encodeFrame({ command: closeCity(mode) }))) return false;
      step({ kind: "closing", mode });
      closing.set(mode);
      return true;
    },
    closing,
    dismissRefusal() {
      store.refused(null);
    },
    markNoticesSeen() {
      store.noticesSeen();
    },
    monitor: watching,
  };
}
