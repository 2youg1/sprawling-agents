<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import type { Snippet } from "svelte";

  import type { LinkState } from "../../core/link";
  import type { Answer, ApprovalItem, AxError, EventRecord, LogLine, Query } from "../../wire";

  interface StandProps {
    readonly link: LinkState;
    readonly unread: readonly AxError[];
    readonly waiting: readonly ApprovalItem[];
    readonly answers?: (query: Query) => Answer | undefined;
    readonly refusing?: AxError | null;
    readonly records?: readonly EventRecord[];
    readonly logs?: readonly LogLine[];
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

  const { link, unread, waiting, answers, refusing, records, logs, children }: StandProps =
    $props();

  const outer = ui();

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

  const kept = createBelief(outer.now);
  // svelte-ignore state_referenced_locally (the belief is fed once, at initialisation, deliberately)
  for (const error of unread) {
    kept.refused(error);
  }
  kept.refused(null);
  $effect(() => {
    if (refusing !== undefined && refusing !== null) kept.refused(refusing);
  });
  kept.batch(() => {
    for (const record of records ?? []) kept.apply(record);
    for (const line of logs ?? []) kept.logged(line);
  });

  function answer(query: Query): Answer | undefined {
    return query === QUERIES.approvals ? { approvals: { items: waiting } } : answers?.(query);
  }

  const stand: Connection = {
    ...outer.conn,
    state: held(() => link),
    belief: kept.belief,
    asking: {
      ...outer.conn.asking,
      ask: (query: Query) => held(() => answer(query)),
      refresh: () => undefined,
    },
    command: () => true,
    retry: () => undefined,
    markNoticesSeen: () => undefined,
  };

  setUi({
    conn: stand,
    prefs: outer.prefs,
    bar: outer.bar,
    origin: outer.origin,
    credential: outer.credential,
    now: outer.now,
  });
</script>

{@render children()}
