// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Option } from "effect";
import { get } from "../core/store";

import { cancel, openSession } from "../core/commands";
import type { Belief } from "../core/belief";
import type { Key, Lang } from "../core/lang";
import { say } from "../core/lang";
import type { Recovery } from "../core/recovering";
import { formOf } from "../core/recovering";
import { current } from "../core/route";
import type { View } from "../core/route";
import { readRunId } from "../core/run_id";
import type { Ui } from "../ui";
import type { Address, AxError, RunId } from "../wire";

export type About = Address | RunId | null;

export interface Refused {
  readonly error: AxError;
  readonly about: About;
}

const VERBS: Readonly<Record<string, Key>> = {
  "/new": "act_open_session",
  "/fork": "act_fork",
};

const NO_TARGET: Key = "act_no_target";

interface Target {
  readonly run: RunId | null;
  readonly room: Address | null;
}

function targetFor(u: Ui, about: About): Target {
  const runs = get(u.conn.belief).runs;
  return {
    run: about === null ? null : Option.getOrNull(readRunId(about)),
    room: Option.match(current(u.bar), {
      onNone: () => null,
      onSome: (view) => composerRoom(view, runs),
    }),
  };
}

function composerRoom(view: View, runs: Belief["runs"]): Address | null {
  if (view.kind === "talk") {
    return view.address;
  }
  if (view.kind === "run") {
    return runs[view.run]?.addr ?? null;
  }
  return null;
}

export function recoveryLabel(recovery: Recovery, lang: Lang): string {
  switch (recovery.kind) {
    case "reconnect":
    case "settings":
    case "reload":
      return say(lang, recovery.verb);
    case "form":
      return say(lang, recovery.label);
    case "command":
      return commandLabel(recovery.spelled, lang);
  }
}

function commandLabel(spelled: string, lang: Lang): string {
  const key = VERBS[spelled];
  return key === undefined ? spelled : say(lang, key);
}

export function recoveryWhy(u: Ui, recovery: Recovery, refused: Refused): Key | undefined {
  if (recovery.kind === "reconnect" || recovery.kind === "settings" || recovery.kind === "reload") {
    return undefined;
  }
  if (recovery.kind === "form") {
    return Option.isNone(formOf(recovery.room, refused.error.subject, "")) ? NO_TARGET : undefined;
  }
  const target = targetFor(u, refused.about);
  if (recovery.spelled === "/stop") {
    return target.run === null ? NO_TARGET : undefined;
  }
  return target.room === null ? NO_TARGET : undefined;
}

export function recover(u: Ui, recovery: Recovery, refused: Refused): void {
  if (recovery.kind === "reconnect") {
    u.conn.retry();
    return;
  }

  if (recovery.kind === "settings") {
    u.go({ kind: "setup", group: "accounts" });
    return;
  }
  if (recovery.kind === "reload") {
    location.reload();
    return;
  }
  if (recovery.kind === "form") {
    const words = say(get(u.lang), recovery.words);
    Option.map(formOf(recovery.room, refused.error.subject, words), (form) => {
      u.prefs.setDraft(form.room, form.draft);
      u.go({ kind: "talk", address: form.room });
    });
    return;
  }
  const target = targetFor(u, refused.about);
  switch (recovery.spelled) {
    case "/new":
      if (target.room !== null) {
        u.send(openSession(target.room, "nothing", null));
      }
      return;
    case "/fork":
      if (target.room !== null) {
        u.go({ kind: "talk", address: target.room });
      }
      return;
    case "/stop":
      if (target.run !== null) {
        u.send(cancel(target.run));
      }
      return;
    default:
      return;
  }
}
