// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createRoot, untrack } from "solid-js";
import { QUERIES } from "../../core/asking";
import { createBelief } from "../../core/belief";
import type { LinkState } from "../../core/link";
import type { PreferenceDoor } from "../../core/prefs";
import { RenderSnippet, type Snippet } from "../../core/snippet";
import type { Connection } from "../../core/socket";
import type { Readable } from "../../core/store";
import { galleryUi, readied, ui, UiScope } from "../../ui";
import type {
  Answer,
  ApprovalItem,
  AxError,
  CommitAnswer,
  EventRecord,
  LogLine,
  Query,
} from "../../wire";
import { commitStateOf, pickCommit } from "../world/chosen";

interface StandProps {
  readonly link: LinkState;
  readonly unread: readonly AxError[];
  readonly waiting: readonly ApprovalItem[];
  readonly answers?: ((query: Query) => Answer | undefined) | undefined;
  readonly refusing?: AxError | null | undefined;
  readonly records?: readonly EventRecord[] | undefined;
  readonly logs?: readonly LogLine[] | undefined;
  readonly commit?: CommitAnswer | undefined;
  readonly prepare?: ((preferences: PreferenceDoor) => void) | undefined;
  readonly children: Snippet;
}

function held<Val>(read: () => Val): Readable<Val> {
  return {
    subscribe(announce) {
      return createRoot((dispose) => {
        announce(untrack(read));
        createEffect(read, (next) => announce(next));
        return dispose;
      });
    },
  };
}

export default function Stand(props: StandProps) {
  const outer = ui();
  const local = galleryUi(outer);
  props.prepare?.(local.prefs);
  const kept = createBelief(outer.now);
  for (const error of untrack(() => props.unread)) kept.refused(error);
  kept.refused(null);
  createEffect(
    () => props.refusing,
    (error) => {
      if (error != null) kept.refused(error);
    },
  );
  kept.batch(() => {
    for (const record of untrack(() => props.records) ?? []) kept.apply(record);
    for (const line of untrack(() => props.logs) ?? []) kept.logged(line);
  });
  function answer(query: Query): Answer | undefined {
    return query === QUERIES.approvals
      ? { approvals: { items: props.waiting } }
      : props.answers?.(query);
  }
  const conn: Connection = {
    ...outer.conn,
    state: held(() => props.link),
    belief: kept.belief,
    asking: {
      ...outer.conn.asking,
      ask: (query) => held(() => answer(query)),
      refresh: () => undefined,
    },
    command: () => true,
    retry: () => undefined,
    markNoticesSeen: () => undefined,
  };
  const fixture = readied({
    conn,
    prefs: local.prefs,
    bar: local.bar,
    origin: local.origin,
    credential: local.credential,
    now: outer.now,
  });
  const selected = untrack(() => props.commit);
  if (selected !== undefined) pickCommit(selected, commitStateOf(fixture));
  return (
    <UiScope value={fixture}>
      <RenderSnippet snippet={props.children} args={[]} />
    </UiScope>
  );
}
