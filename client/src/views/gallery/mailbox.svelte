<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The mailbox (client-SPEC 4-49): its key in each of the ways it is
  // marked, its column with every section holding something and with
  // nothing in it, and the decide card in the conversation's width. The
  // key and the column read the city instead of their props, so each
  // fixture hands them one through `Stand`.
  import type { Answer, AxError, EventRecord, Query, SessionLine } from "../../wire";
  import { Address, B3Hash, RunId, Seq, TimeMs } from "../../wire";

  function refusal(code: AxError["code"], action: string, subject: string, recovery: string): AxError {
    return { action, code, gate: null, nearby: [], recovery, retry: "no", subject };
  }

  // One of each kind the mailbox sorts: a door waiting on the person's
  // hand, a failure that stops the work, and an ordinary refusal.
  const ASKS = refusal(
    "E_APPROVAL_PENDING",
    "attach to the browser you are signed into",
    "lab/east",
    "open the browser and press allow on the attach prompt",
  );
  const STOPS = refusal("E_MODEL_UNCHOSEN", "dispatch to hall/mayor", "main", "choose a main model in settings");
  const ORDINARY = refusal("E_PATH_NOT_FOUND", "read a file", "lab/east/docs/SPEC.md", "check the path and ask again");

  // The fixture's clock: its runs began a few minutes before the page
  // opened, so the times they show read the way a working city reads.
  const NOW = Date.now();
  const MINUTE = 60_000;

  const RUN_A = RunId.make("0199c0de-0000-4c3d-8e4f-000000000001");
  const RUN_B = RunId.make("0199c0de-0000-4c3d-8e4f-000000000002");
  const RUN_C = RunId.make("0199c0de-0000-4c3d-8e4f-000000000003");
  const EAST = Address.make("lab/east");
  const WEST = Address.make("lab/west");
  const MAYOR = Address.make("hall/mayor");

  function record(run: RunId, seq: number, ago: number, kind: EventRecord["kind"], addr: Address, data: Record<string, unknown>): EventRecord {
    return {
      addr,
      data,
      kind,
      prev: B3Hash.make("0".repeat(64)),
      run,
      seq: Seq.make(seq),
      t: TimeMs.make(NOW - ago),
      v: 1,
      who: addr,
    };
  }

  // Two rooms at work - one calling a tool, one waiting on the person -
  // and the mayor's room, whose run has frozen.
  const RECORDS: readonly EventRecord[] = [
    record(RUN_C, 1, 40 * MINUTE, "run_started", MAYOR, { task: "plan the documents crate" }),
    record(RUN_C, 2, 31 * MINUTE, "run_frozen", MAYOR, { completion: "done" }),
    record(RUN_A, 3, 6 * MINUTE, "run_started", EAST, { task: "make the document reader lossless" }),
    record(RUN_A, 4, 1 * MINUTE, "tool_called", EAST, { name: "exec", subject: "cargo nextest -p documents" }),
    record(RUN_B, 5, 3 * MINUTE, "run_started", WEST, { task: "write the migration notes" }),
    record(RUN_B, 6, 2 * MINUTE, "approval_requested", WEST, { action_desc: "exec: rm -rf target" }),
  ];

  function line(began: number, last: number, ago: number, runs: number, start: SessionLine["start"]): SessionLine {
    return { at: TimeMs.make(NOW - ago), began: Seq.make(began), last: Seq.make(last), runs, start };
  }

  // Each room's sessions, newest first, as `Query::Sessions` answers.
  const SESSIONS: Readonly<Record<string, readonly SessionLine[]>> = {
    [MAYOR]: [
      line(1, 2, 31 * MINUTE, 1, { opened: { carry: "handoff", from: null } }),
      line(0, 0, 3 * 60 * MINUTE, 4, "dispatched"),
    ],
    [EAST]: [line(3, 4, 1 * MINUTE, 1, { opened: { carry: "nothing", from: { run: RUN_C, at_seq: Seq.make(2) } } })],
    [WEST]: [line(5, 6, 2 * MINUTE, 1, "dispatched")],
  };

  function sessions(query: Query): Answer | undefined {
    if (typeof query !== "object" || !("sessions" in query)) return undefined;
    const room = query.sessions.room;
    return { sessions: { room, sessions: SESSIONS[room] ?? [], earlier: room === MAYOR ? 12 : 0 } };
  }
</script>

<script lang="ts">
  import Column from "../mailbox/column.svelte";
  import Mailbox from "../mailbox/mailbox.svelte";
  import Case from "./case.svelte";
  import { ONE_QUESTION } from "./served";
  import Stand from "./stand.svelte";
  import WaitingCards from "../talk/waiting_cards.svelte";
  import Deciding from "../mailbox/deciding.svelte";

  const LIVE = { kind: "live", city: "sprawling" } as const;
  const COLUMN = "flex h-[760px] flex-col bg-raised";
  const ignore = (): void => undefined;
</script>

<Case label="mailbox key · nothing to do">
  <Stand link={LIVE} unread={[]} waiting={[]}>
    <Mailbox asked={0} hint={(words: string) => words} />
  </Stand>
</Case>

<Case label="mailbox key · something new and nothing waiting">
  <Stand link={LIVE} unread={[ORDINARY]} waiting={[]}>
    <Mailbox asked={0} hint={(words: string) => words} />
  </Stand>
</Case>

<Case label="mailbox key · a question and a stopped city waiting">
  <Stand link={LIVE} unread={[STOPS, ORDINARY]} waiting={[ONE_QUESTION]}>
    <Mailbox asked={0} hint={(words: string) => words} />
  </Stand>
</Case>

<!-- Still on its way up, the state that resolves by itself: the bar
pulses rather than asking for anything. -->
<Case label="mailbox key · the link is still connecting">
  <Stand link={{ kind: "handshaking" }} unread={[]} waiting={[]}>
    <Mailbox asked={0} hint={(words: string) => words} />
  </Stand>
</Case>

<Case label="mailbox · every section holding something" width={440}>
  <Stand link={LIVE} unread={[ASKS, STOPS, ORDINARY]} waiting={[ONE_QUESTION]} answers={sessions} records={RECORDS}>
    <div class={COLUMN}>
      <Column onClose={ignore} />
    </div>
  </Stand>
</Case>

<Case label="mailbox · nothing in it, the link connecting" width={440}>
  <Stand link={{ kind: "backoff", attempt: 2 }} unread={[]} waiting={[]}>
    <div class="flex h-[480px] flex-col bg-raised">
      <Column onClose={ignore} />
    </div>
  </Stand>
</Case>

<Case label="decide card · a design question in the conversation">
  <Stand link={LIVE} unread={[]} waiting={[]}>
    <WaitingCards items={[ONE_QUESTION]} />
  </Stand>
</Case>

<Case label="decide card · a door waiting on the person's hand">
  <Stand link={LIVE} unread={[ASKS]} waiting={[]}>
    <Deciding />
  </Stand>
</Case>
