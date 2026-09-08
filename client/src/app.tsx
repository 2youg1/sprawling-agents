// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A placeholder that proves the pipeline end to end: one phrase from the
// language table and the current route, redrawn on every `hashchange`.
// The screens themselves arrive with card 6.3 onward.

import { Option } from "effect";
import { createSignal, onCleanup, onMount } from "solid-js";

import { say, type Lang } from "./core/lang";
import {
  DEFAULT_VIEW,
  current,
  toFragment,
  type AddressBar,
  type View,
} from "./core/route";

export interface AppProps {
  readonly bar: Readonly<AddressBar>;
  readonly lang: Lang;
}

export function App(props: AppProps) {
  const [view, setView] = createSignal<View>(DEFAULT_VIEW);
  const follow = () => {
    setView(Option.getOrElse(current(props.bar), () => DEFAULT_VIEW));
  };
  onMount(() => {
    follow();
    window.addEventListener("hashchange", follow);
  });
  onCleanup(() => {
    window.removeEventListener("hashchange", follow);
  });
  return (
    <main class="min-h-screen bg-g0 p-section font-sans text-body text-text">
      <h1 class="text-title font-title">{say(props.lang, "nav_sessions")}</h1>
      <p class="mt-snug font-mono text-note text-text-quiet">
        {toFragment(view())}
      </p>
    </main>
  );
}
