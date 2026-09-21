// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A city to stand a fixture in, for the components that read one.
//
// Nearly every fixture on this route takes its state through props,
// which is the whole reason a component's state is a prop. One does
// not: the dot at the top of the rail reads the link, the questions
// waiting and the refusals nobody has opened, all three through the
// context every view is handed - so drawing it in a state worth
// looking at means handing that subtree a city, and this is the one
// place on this route where a city is made up.
//
// **What the fixture does not state stays the real one's.** The
// language, the address bar, the origin and the clock come from the
// provider above, so a fixture reads Chinese on a Chinese page and a
// caption here never becomes a second answer to a question `ui.tsx`
// already answers.
//
// **Nothing here can reach the running city.** Retrying the link and
// marking refusals read are the two verbs the dot offers, and both are
// replaced: a fixture that can act on the city is a fixture that can
// change it, and a gallery that changed the city would be measuring
// something it had just altered.

import { onMount, type JSX } from "solid-js";

import { QUERIES } from "../../core/asking";
import { createBelief } from "../../core/belief";
import type { Belief } from "../../core/belief";
import type { LinkState } from "../../core/link";
import type { Answer, ApprovalItem, AxError, Query } from "../../wire";
import { UiProvider, useUi } from "../../ui";
import type { Ui } from "../../ui";

export interface StandProps {
  // Where the socket stands, which is one of the three things that
  // turn the rail's dot yellow.
  readonly link: LinkState;
  // The refusals this session has seen and nobody has opened.
  readonly unread: readonly AxError[];
  // The questions this city is holding for the person.
  readonly waiting: readonly ApprovalItem[];
  // What this made-up city answers besides the questions waiting. A
  // fixture that needs a second answer writes the match itself,
  // because only it knows which question it meant.
  readonly answers?: (query: Query) => Answer | undefined;
  readonly children: JSX.Element;
}

export function Stand(props: StandProps) {
  const outer = useUi();
  // The belief is the constructor's own empty value, and the refusals
  // this stand was handed go in through the belief's own door - inside
  // a tracked scope, so a fixture that changes the list after mounting
  // gets the notice the same way a live page does. A fixture that
  // spelled the empty value here would be right about the keys and
  // wrong about every value the moment one changes, and nothing would
  // fail.
  const store = createBelief();
  onMount(() => {
    for (const error of props.unread) store.refused(error);
    store.refused(null);
  });
  const belief: Belief = store.belief;
  // The question every stand-in answers, and the fixture's own.
  // Anything neither of them names is `undefined`, which is what a
  // page that has asked and not yet been answered holds, and what
  // every other fixture on this route already draws under.
  const answer = (query: Query): Answer | undefined =>
    query === QUERIES.approvals
      ? { approvals: { items: props.waiting } }
      : props.answers?.(query);
  const value: Ui = {
    ...outer,
    conn: {
      ...outer.conn,
      state: () => props.link,
      belief,
      asking: { ...outer.conn.asking, ask: (query) => () => answer(query) },
      retry: () => undefined,
      markNoticesSeen: () => undefined,
    },
  };
  return <UiProvider value={value}>{props.children}</UiProvider>;
}
