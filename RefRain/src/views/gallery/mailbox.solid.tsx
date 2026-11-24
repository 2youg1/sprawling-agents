// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createUniqueId, For } from "solid-js";
import { storeValue } from "../../core/solid_store";
import { ago } from "../../core/time";
import { ui } from "../../ui";
import Deciding from "../mailbox/deciding";
import Mailbox, { MailboxColumn as Column } from "../mailbox/mailbox";
import { RecentRow } from "../mailbox/recent";
import { rowOf as recentRow } from "../mailbox/recent_row";
import { WorkingRow } from "../mailbox/working";
import { rowOf as workingRow } from "../mailbox/working_row";
import WaitingCards from "../talk/waiting_view";
import { GalleryCase } from "./case";
import { ONE_QUESTION } from "./served";
import Stand from "./stand.solid";

import type { Answer, AxError, EventRecord, Query, SessionLine } from "../../wire";
import { Address, B3Hash, RunId, Seq, TimeMs } from "../../wire";
import type { BranchHere } from "../talk/branching";

function refusal(
  code: AxError["code"],
  action: string,
  subject: string,
  recovery: string,
): AxError {
  return { action, code, gate: null, nearby: [], recovery, retry: "no", subject };
}

const ASKS = refusal(
  "E_APPROVAL_PENDING",
  "attach to the browser you are signed into",
  "lab/east",
  "open the browser and press allow on the attach prompt",
);
const STOPS = refusal(
  "E_MODEL_UNCHOSEN",
  "dispatch to hall/mayor",
  "main",
  "choose a main model in settings",
);
const ORDINARY = refusal(
  "E_PATH_NOT_FOUND",
  "read a file",
  "lab/east/docs/SPEC.md",
  "check the path and ask again",
);
const BUSY = refusal(
  "E_BUSY",
  "start a new session",
  "hall/mayor: a run is running there",
  "stop that run first",
);
const BRANCHES: readonly BranchHere[] = [
  { kind: "opening", of: "branch" },
  { kind: "refused", of: "branch", error: BUSY },
];

const TURNED_AWAY = {
  kind: "refused",
  error: refusal(
    "E_WIRE_MISMATCH",
    "open the link to the city",
    "wire",
    "reload the page to fetch the client this city serves",
  ),
} as const;

const NOW = Date.now();
const MINUTE = 60_000;

const RUN_A = RunId.make("0199c0de-0000-4c3d-8e4f-000000000001");
const RUN_B = RunId.make("0199c0de-0000-4c3d-8e4f-000000000002");
const RUN_C = RunId.make("0199c0de-0000-4c3d-8e4f-000000000003");
const EAST = Address.make("lab/east");
const WEST = Address.make("lab/west");
const MAYOR = Address.make("hall/mayor");

function record(
  run: RunId,
  seq: number,
  ago: number,
  kind: EventRecord["kind"],
  addr: Address,
  data: Record<string, unknown>,
): EventRecord {
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

const RECORDS: readonly EventRecord[] = [
  record(RUN_C, 1, 40 * MINUTE, "run_started", MAYOR, { task: "plan the documents crate" }),
  record(RUN_C, 2, 31 * MINUTE, "run_frozen", MAYOR, { completion: "done" }),
  record(RUN_A, 3, 6 * MINUTE, "run_started", EAST, {
    task: "make the document reader lossless",
  }),
  record(RUN_A, 4, 1 * MINUTE, "tool_called", EAST, {
    name: "exec",
    subject: "cargo nextest -p documents",
  }),
  record(RUN_B, 5, 3 * MINUTE, "run_started", WEST, { task: "write the migration notes" }),
  record(RUN_B, 6, 2 * MINUTE, "approval_requested", WEST, {
    action_desc: "exec: rm -rf target",
  }),
];

function line(
  began: number,
  last: number,
  ago: number,
  runs: number,
  start: SessionLine["start"],
): SessionLine {
  return {
    at: TimeMs.make(NOW - ago),
    began: Seq.make(began),
    last: Seq.make(last),
    runs,
    start,
  };
}

const SESSIONS: Readonly<Record<string, readonly SessionLine[]>> = {
  [MAYOR]: [
    line(1, 2, 31 * MINUTE, 1, { opened: { carry: "handoff", from: null } }),
    line(0, 0, 3 * 60 * MINUTE, 4, { dispatched: { by: null } }),
  ],
  [EAST]: [
    line(3, 4, 1 * MINUTE, 1, {
      opened: { carry: "nothing", from: { run: RUN_C, at_seq: Seq.make(2) } },
    }),
  ],
  [WEST]: [line(5, 6, 2 * MINUTE, 1, { dispatched: { by: null } })],
};

function sessions(query: Query): Answer | undefined {
  if (typeof query !== "object" || !("sessions" in query)) return undefined;
  const room = query.sessions.room;
  return { sessions: { room, sessions: SESSIONS[room] ?? [], earlier: room === MAYOR ? 12 : 0 } };
}
const LIVE = { kind: "live", city: "sprawling" } as const;
const COLUMN = "flex h-[1480px] flex-col bg-raised";
const ignore = (): void => undefined;

export default function MailboxCases() {
  const lang = storeValue(ui().lang);
  return (
    <>
      <GalleryCase label="mailbox key · nothing to do">
        <Stand
          link={LIVE}
          unread={[]}
          waiting={[]}
          children={() => (
            <>
              <Mailbox asked={0} keyId={createUniqueId()} hint={(words) => words} />
            </>
          )}
        />
      </GalleryCase>

      <GalleryCase label="mailbox key · something new and nothing waiting">
        <Stand
          link={LIVE}
          unread={[ORDINARY]}
          waiting={[]}
          children={() => (
            <>
              <Mailbox asked={0} keyId={createUniqueId()} hint={(words) => words} />
            </>
          )}
        />
      </GalleryCase>

      <GalleryCase label="mailbox key · a question and a stopped city waiting">
        <Stand
          link={LIVE}
          unread={[STOPS, ORDINARY]}
          waiting={[ONE_QUESTION]}
          children={() => (
            <>
              <Mailbox asked={0} keyId={createUniqueId()} hint={(words) => words} />
            </>
          )}
        />
      </GalleryCase>

      <GalleryCase label="mailbox key · the link is still connecting">
        <Stand
          link={{ kind: "handshaking" }}
          unread={[]}
          waiting={[]}
          children={() => (
            <>
              <Mailbox asked={0} keyId={createUniqueId()} hint={(words) => words} />
            </>
          )}
        />
      </GalleryCase>

      <GalleryCase label="mailbox key · the city refused the page">
        <Stand
          link={TURNED_AWAY}
          unread={[STOPS]}
          waiting={[]}
          children={() => (
            <>
              <Mailbox asked={0} keyId={createUniqueId()} hint={(words) => words} />
            </>
          )}
        />
      </GalleryCase>

      <GalleryCase label="mailbox · every section holding something" width={440}>
        <Stand
          link={LIVE}
          unread={[ASKS, STOPS, ORDINARY]}
          waiting={[ONE_QUESTION]}
          answers={sessions}
          records={RECORDS}
          children={() => (
            <>
              <div class={COLUMN}>
                <Column onClose={ignore} />
              </div>
            </>
          )}
        />
      </GalleryCase>

      <GalleryCase label="mailbox · nothing in it, the link connecting" width={440}>
        <Stand
          link={{ kind: "backoff", attempt: 2 }}
          unread={[]}
          waiting={[]}
          children={() => (
            <>
              <div class="flex h-[480px] flex-col bg-raised">
                <Column onClose={ignore} />
              </div>
            </>
          )}
        />
      </GalleryCase>

      <GalleryCase label="mailbox · the city refused the page, with the way to mend it" width={440}>
        <Stand
          link={TURNED_AWAY}
          unread={[]}
          waiting={[]}
          children={() => (
            <>
              <div class="flex h-[480px] flex-col bg-raised">
                <Column onClose={ignore} />
              </div>
            </>
          )}
        />
      </GalleryCase>

      <GalleryCase
        label="mailbox rows · recent sessions, one that forks and one that cannot"
        width={440}
      >
        <ul class="bg-raised px-base">
          <For each={[true, false]}>
            {(forkable) => {
              const line = SESSIONS[MAYOR]?.[forkable ? 0 : 1];
              return line === undefined ? null : (
                <RecentRow
                  {...recentRow(
                    { room: MAYOR, line, forkable, branch: { kind: "none" } },
                    lang(),
                    ago(lang(), line.at, NOW),
                    {
                      href: "#/gallery",
                      follow: ignore,
                      fork: ignore,
                      stop: null,
                    },
                  )}
                />
              );
            }}
          </For>
        </ul>
      </GalleryCase>

      <GalleryCase
        label="mailbox rows · a branch opening and one refused under a working run"
        width={440}
      >
        <ul class="bg-raised px-base">
          <For each={BRANCHES}>
            {(branch) => {
              const line = SESSIONS[MAYOR]?.[0];
              return line === undefined ? null : (
                <RecentRow
                  {...recentRow(
                    { room: MAYOR, line, forkable: true, branch },
                    lang(),
                    ago(lang(), line.at, NOW),
                    {
                      href: "#/gallery",
                      follow: ignore,
                      fork: ignore,
                      stop: ignore,
                    },
                  )}
                />
              );
            }}
          </For>
        </ul>
      </GalleryCase>

      <GalleryCase
        label="mailbox rows · a room calling a tool and one waiting on the person"
        width={440}
      >
        <ul class="bg-raised px-base">
          <WorkingRow
            {...workingRow(
              {
                doing: { kind: "calling", tool: "exec", subject: null },
                started: TimeMs.make(NOW - 6 * MINUTE),
                task: "make the document reader lossless",
              },
              EAST,
              lang(),
              NOW,
              { href: "#/gallery", onclick: ignore },
            )}
          />
          <WorkingRow
            {...workingRow(
              { doing: { kind: "waiting" }, started: TimeMs.make(NOW - 3 * MINUTE), task: null },
              WEST,
              lang(),
              NOW,
              {
                href: "#/gallery",
                onclick: ignore,
              },
            )}
          />
        </ul>
      </GalleryCase>

      <GalleryCase label="decide card · a design question in the conversation">
        <Stand
          link={LIVE}
          unread={[]}
          waiting={[]}
          children={() => (
            <>
              <WaitingCards items={[ONE_QUESTION]} />
            </>
          )}
        />
      </GalleryCase>

      <GalleryCase label="decide card · a door waiting on the person's hand">
        <Stand
          link={LIVE}
          unread={[ASKS]}
          waiting={[]}
          children={() => (
            <>
              <Deciding onLeave={ignore} />
            </>
          )}
        />
      </GalleryCase>
    </>
  );
}
