// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The browser half of the link: one socket, three listeners, and no
// decisions. Every judgement is `link.ts`'s; this turns browser
// callbacks into link events, carries out the actions the machine
// answers with, and hands what arrives to the belief and the asking.
//
// Frames are folded once per animation frame rather than as they land:
// a burst of records becomes one store update and one paint, which is
// what keeps a busy city under the 16 ms the person asked for.

import { batch, createSignal } from "solid-js";
import type { Accessor } from "solid-js";

import { createAsking } from "./asking";
import type { Asking } from "./asking";
import { createBelief } from "./belief";
import type { Belief } from "./belief";
import { decodeFrame, encodeFrame } from "./frames";
import { advance, connect as start, isLive, newLink } from "./link";
import type { Link, LinkAction, LinkEvent, LinkState } from "./link";
import type { Command, Query, ServerFrame } from "../wire";

export interface Connection {
  readonly state: Accessor<LinkState>;
  readonly belief: Belief;
  readonly asking: Asking;
  // Sends one command; false when the link is not live, in which case
  // nothing was sent and the person should be told rather than left to
  // wonder.
  readonly command: (command: Command) => boolean;
  readonly retry: () => void;
  readonly dismissRefusal: () => void;
  // Everything in the bell has now been looked at.
  readonly markNoticesSeen: () => void;
}

// The pairing code the host put on the URL that opened this page. An
// empty value is not a value.
export function tokenIn(search: string): string | null {
  const value = new URLSearchParams(search).get("token");
  return value === null || value === "" ? null : value;
}

// The address of this city's socket, derived from the page's own origin:
// a client served by the city it talks to needs no configured endpoint.
export function socketUrl(location: Location): string {
  const scheme = location.protocol === "https:" ? "wss" : "ws";
  return `${scheme}://${location.host}/ws`;
}

export function openConnection(url: string, token: string | null): Connection {
  let link: Link = newLink(token);
  let socket: WebSocket | null = null;
  const [state, setState] = createSignal<LinkState>(link.state);
  const store = createBelief();

  const queue: ServerFrame[] = [];
  let scheduled = false;

  function sendText(text: string): boolean {
    if (socket?.readyState !== WebSocket.OPEN) {
      return false;
    }
    socket.send(text);
    return true;
  }

  const asking = createAsking((query: Query) =>
    isLive(link) ? sendText(encodeFrame({ query })) : false,
  );

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
        asking.reconnected();
        return;
      case "deliver":
        store.apply(action.event);
        asking.invalidate(action.event);
        return;
      case "answered":
        if ("city" in action.answer) {
          store.adoptCity(action.answer.city);
        }
        asking.answered(action.answer);
        return;
      case "saying":
        store.say(action.delta);
        return;
      case "logged":
        store.logged(action.line);
        return;
      case "wait":
        setTimeout(() => {
          step({ kind: "wait_elapsed" });
        }, action.ms);
        return;
      case "report":
        store.refused(action.error);
        return;
      case "close":
        socket?.close();
        return;
    }
  }

  function step(event: LinkEvent): void {
    const [next, action] = advance(link, event);
    link = next;
    if (next.state !== state()) {
      setState(next.state);
    }
    perform(action);
  }

  // Drains what arrived since the last paint, in order, as one update.
  function drain(): void {
    scheduled = false;
    const frames = queue.splice(0, queue.length);
    batch(() => {
      for (const frame of frames) {
        step({ kind: "received", frame });
      }
    });
  }

  function open(): void {
    const opened = new WebSocket(url);
    socket = opened;
    opened.onopen = () => {
      if (socket === opened) step({ kind: "opened" });
    };
    opened.onmessage = (message: MessageEvent) => {
      if (socket !== opened || typeof message.data !== "string") {
        return;
      }
      const frame = decodeFrame(message.data);
      if (frame === null) {
        // The two ends disagree about the wire; the machine knows what
        // a closed link means.
        opened.close();
        return;
      }
      // A welcome is folded at once, so the questions the page has been
      // holding go out on the same tick the link comes up.
      if ("welcome" in frame || "refusal" in frame) {
        drain();
        step({ kind: "received", frame });
        return;
      }
      queue.push(frame);
      if (!scheduled) {
        scheduled = true;
        requestAnimationFrame(drain);
      }
    };
    const closed = () => {
      if (socket !== opened) return;
      socket = null;
      step({ kind: "closed" });
    };
    opened.onclose = closed;
    opened.onerror = closed;
  }

  const [begun, first] = start(link);
  link = begun;
  setState(begun.state);
  perform(first);

  return {
    state,
    belief: store.belief,
    asking,
    command(command) {
      return isLive(link) && sendText(encodeFrame({ command }));
    },
    retry() {
      step({ kind: "retry" });
    },
    dismissRefusal() {
      store.refused(null);
    },
    markNoticesSeen() {
      store.noticesSeen();
    },
  };
}
