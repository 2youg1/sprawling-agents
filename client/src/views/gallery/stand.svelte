<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // A city to stand a fixture in, for the components that read one.
  //
  // Nearly every fixture on this route takes its state through props,
  // which is the whole reason a component's state is a prop. One does
  // not: the dot at the top of the rail reads the link, the questions
  // waiting and the refusals nobody has opened, all three through the
  // door every view is handed (`src/ui.ts`) - so drawing it in a state
  // worth looking at means handing that subtree a city, and this is
  // the one place on this route where a city is made up.
  //
  // **The door is a single one, so the stand installs on it while its
  // subtree initialises and `gallery.svelte` closes it once the route
  // has mounted.** A component captures `ui()` at its own
  // initialisation, so every reader inside this subtree is handed this
  // stand's city before the door is handed back; a reader arriving
  // later than that meets the real city again. The install replaces
  // exactly the five readings the dot takes off the connection - where
  // the socket stands, the notices kept, the questions asked, and the
  // two verbs below - and inherits the rest from the city before it,
  // which is why stands nest correctly: one stand inside another
  // converges on the same preferences, address bar and clock.
  //
  // **What the fixture does not state stays the real one's.** The
  // language, the address bar, the origin and the clock come from the
  // city above, so a fixture reads Chinese on a Chinese page and a
  // caption here never becomes a second answer to a question `ui.ts`
  // already answers.
  //
  // **Nothing here can reach the running city.** Retrying the link and
  // marking refusals read are the two verbs the dot offers, and both
  // are replaced: a fixture that can act on the city is a fixture that
  // can change it, and a gallery that changed the city would be
  // measuring something it had just altered.

  import type { Snippet } from "svelte";

  import type { LinkState } from "../../core/link";
  import type { Answer, ApprovalItem, AxError, Query } from "../../wire";

  interface StandProps {
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
    // A refusal that arrives after mounting, the way a command's answer
    // does: set it from inside the fixture to answer what it sent.
    readonly refusing?: AxError | null;
    readonly children: Snippet;
  }
</script>

<script lang="ts">
  import { untrack } from "svelte";
  import type { Readable, Subscriber } from "svelte/store";

  import { QUERIES } from "../../core/asking";
  import { createBelief } from "../../core/belief";
  import type { Connection } from "../../core/socket";
  import { setUi, ui } from "../../ui";

  const { link, unread, waiting, answers, refusing, children }: StandProps = $props();

  // The city as this fixture meets it: the real one, or the stand
  // before this one while several stands mount in turn.
  const outer = ui();

  // A store over a prop read at subscribe time and again whenever the
  // prop moves. The store contract is a `subscribe` that answers
  // synchronously and again on every change, which is exactly what a
  // prop offers through its getter - this is the bridge between the
  // two, so a fixture whose subject moves after mounting is followed
  // the same way a live page follows the socket.
  function held<A>(read: () => A): Readable<A> {
    return {
      subscribe(run: Subscriber<A>): () => void {
        run(read());
        return $effect.root(() => {
          $effect(() => {
            const value = read();
            untrack(() => {
              run(value);
            });
          });
        });
      },
    };
  }

  // The belief is the constructor's own empty value, and the refusals
  // this stand was handed go in through the belief's own door, so a
  // fixture gets the notice the same way a live page does. A fixture
  // that spelled the empty value here would be right about the keys
  // and wrong about every value the moment one changes, and nothing
  // would fail. The read of `unread` below is meant to capture the
  // values the fixture mounted with: a stand answers from what it was
  // mounted with, and nothing on this route moves those values
  // afterwards.
  const kept = createBelief(outer.now);
  // svelte-ignore state_referenced_locally (the belief is fed once, at initialisation, deliberately)
  for (const error of unread) {
    kept.refused(error);
  }
  kept.refused(null);
  $effect(() => {
    if (refusing !== undefined && refusing !== null) kept.refused(refusing);
  });

  // The question every stand-in answers, and the fixture's own.
  // Anything neither of them names is `undefined`, which is what a
  // page that has asked and not yet been answered holds, and what
  // every other fixture on this route already draws under.
  function answer(query: Query): Answer | undefined {
    return query === QUERIES.approvals
      ? { approvals: { items: waiting } }
      : answers?.(query);
  }

  const stand: Connection = {
    ...outer.conn,
    state: held(() => link),
    belief: kept.belief,
    asking: {
      ...outer.conn.asking,
      ask: (query: Query) => held(() => answer(query)),
    },
    retry: () => undefined,
    markNoticesSeen: () => undefined,
  };

  // Installed here and nowhere else: `ui.ts` has one door, the shell
  // opens it as it mounts, and this re-opens it for the length of one
  // subtree's initialisation. `gallery.svelte` closes it.
  setUi({
    conn: stand,
    prefs: outer.prefs,
    bar: outer.bar,
    origin: outer.origin,
    pairing: outer.pairing,
    now: outer.now,
  });
</script>

{@render children()}
