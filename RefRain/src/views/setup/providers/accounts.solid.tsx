// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, flush, onCleanup } from "solid-js";
import { enrol } from "../../../core/enrol";
import { storeValue } from "../../../core/solid_store";
import { ui } from "../../../ui";
import {
  awaits,
  freshEditor,
  leave,
  refuse,
  settle,
  type Hands,
  type Roster,
  type Step,
} from "./account_editor";
import { lookOf, type Holds } from "./accounts";
import Look from "./accounts.look.solid";
import { editorState } from "./editor_state";
import type { FlightState } from "./flight";

export interface AccountsProps {
  readonly roster: Roster;
  readonly flight?: FlightState | undefined;
}

export default function Accounts(props: AccountsProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const editor = editorState(freshEditor());
  let heading: HTMLElement | undefined;
  const held = new Map<string, HTMLElement>();

  function afterLayout(focus: () => void): void {
    queueMicrotask(() => {
      flush();
      focus();
    });
  }
  const hands = (): Hands => ({
    roster: () => props.roster,
    lang,
    reach: () => ({ origin: u.origin, token: u.credential() }),
    send: (command) => u.send(command),
    enrol,
    follow: (id, step) =>
      afterLayout(() => held.get(`${id}:${step}`)?.querySelector("button")?.focus()),
    focusHeading: () => afterLayout(() => heading?.focus()),
    ...(props.flight === undefined ? {} : { flight: props.flight }),
  });
  const holds: Holds = {
    heading: (node) => {
      heading = node;
      return () => {
        if (heading === node) heading = undefined;
      };
    },
    control: (id: string, step: Step) => (node) => {
      const key = `${id}:${step}`;
      held.set(key, node);
      return () => {
        if (held.get(key) === node) held.delete(key);
      };
    },
  };
  createEffect(
    () => [props.roster, editor.sent] as const,
    ([roster]) => settle(editor, roster),
  );
  createEffect(
    () => belief().refusal,
    (why) => {
      if (why === null || !awaits(editor)) return;
      refuse(editor, why);
      u.conn.dismissRefusal();
    },
  );
  onCleanup(() => leave(editor));
  const look = createMemo(() => lookOf(editor, hands(), holds));
  return <Look {...look()} />;
}
