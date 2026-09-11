// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Every state worth looking at, on fixtures, with no city behind it.
//
// It is a route rather than a build flag for two reasons: the gate that
// measures it opens the same bundle a person runs, and a person deciding
// between two ways a thing could look can open it on their own machine
// without standing up a provider first.

import { For, type JSX } from "solid-js";

import { sendingInto, type Doing, type Sending } from "../core/belief";
import { useSay } from "../ui";
import { Composer } from "./talk/composer";

// The postures a run can be in, in the order a dispatch meets them.
const POSTURES: readonly Doing[] = [
  { kind: "thinking" },
  { kind: "calling", tool: "exec", subject: "just check" },
  { kind: "waiting" },
  { kind: "frozen", completion: "done" },
];

// How many characters of a live turn are drawn faint. Kept in step with
// `thread.tsx` by being shown here at several lengths rather than by a
// second copy of the number: what this page is for is deciding whether
// the number is right.
const SAYING = [
  "on",
  "on it — reading the city",
  "on it — reading the city, then writing the plan it asks for",
];

function Case(props: { readonly label: string; readonly children: JSX.Element }) {
  return (
    <section class="mb-wide" aria-label={props.label}>
      <div class="mb-tight text-note text-text-disabled">{props.label}</div>
      {props.children}
    </section>
  );
}

export function Gallery() {
  const say = useSay();
  return (
    <div class="mx-auto w-full max-w-talk px-pane py-pane">
      <h1 class="mb-wide text-heading text-text">{say("gallery_title")}</h1>

      <For each={POSTURES}>
        {(doing) => (
          <Case label={`${doing.kind} · ${sendingInto(doing)}`}>
            <Composer
              placeholder={say("talk_placeholder_mayor")}
              sending={sendingInto(doing) satisfies Sending}
              onSend={() => false}
              onStop={() => false}
            />
          </Case>
        )}
      </For>

      <For each={SAYING}>
        {(text) => (
          <Case label={`saying · ${String(text.length)}`}>
            <div class="whitespace-pre-wrap leading-relaxed text-body">
              {text.slice(0, -10)}
              <span class="text-text-faint">{text.slice(-10)}</span>
              <span class="ml-tight inline-block h-caret w-hair animate-pulse bg-accent align-text-bottom" />
            </div>
          </Case>
        )}
      </For>
    </div>
  );
}
