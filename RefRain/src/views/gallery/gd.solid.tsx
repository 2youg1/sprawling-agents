// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For } from "solid-js";
import { Address, RunId, Seq, TimeMs } from "../../wire";
import type { Answer, Call, FindAnswer, Query } from "../../wire";
const LAB = Address.make("lab");
const RUN = RunId.make("0199c0de-7700-4000-8000-0000000000d4");
const FOUND: readonly Address[] = [
  "Roadmap.md",
  "notes/roadmap-review.md",
  "notes/2026/roadmap-q3.md",
  "docs/archive/old-roadmap.txt",
].map((path) => Address.make(path));
function finding(walked: FindAnswer["walked"]): (query: Query) => Answer | undefined {
  return (query) =>
    typeof query === "object" && "find" in query
      ? { find: { under: query.find.under, text: query.find.text, paths: [...FOUND], walked } }
      : undefined;
}
function call(at: number, tool: string, subject: string, took: number): Call {
  const called = 1_790_000_000_000 + at * 1000;
  return {
    tool,
    subject,
    arguments: null,
    outcome: "answered",
    at: Seq.make(at),
    output: null,
    called: TimeMs.make(called),
    answered: TimeMs.make(called + took),
    timing: "measured",
    effect: "read",
    render: "generic",
  };
}
const REFUSAL = JSON.stringify({
  code: "E_PATH_NOT_FOUND",
  action: "read",
  subject: "crates/nowhere/src/lib.rs",
  nearby: ["Sprawling/city/src/lib.rs"],
  recovery: "list the parent directory first",
  retry: "no",
});
const FAILED: readonly Call[] = [
  {
    ...call(51, "read", "crates/nowhere/src/lib.rs", 4),
    outcome: "failed",
    output: { head: REFUSAL, cut: 0 },
  },
  {
    ...call(52, "fetch_docs", "docs.rs/git2/latest/git2/struct.Repository.html", 2_210),
    effect: "egress",
    outcome: "failed",
    output: {
      head: "upstream answered 503 Service Unavailable after three attempts; the mirror was not tried\nretry later",
      cut: 0,
    },
  },
  {
    ...call(53, "browser", "screenshot", 880),
    effect: "egress",
    outcome: "failed",
    output: null,
  },
];
const CALLS: readonly Call[] = [
  call(41, "read", "Sprawling/city/src/document.rs", 31),
  call(42, "search", "edit_against", 140),
  call(43, "exec", "cargo nextest -p city", 3_412),
];
import CallLine from "../talk/call_line_view";
import Search from "../finder/search.solid";
import { GalleryCase } from "./case";
import Stand from "./stand.solid";

export default function CallAndFinderCases() {
  return (
    <>
      <For
        each={Array.from(["whole", "cut"] as const).map((__rrItem0, __rrIndex1) => ({
          value: __rrItem0,
          key: __rrItem0,
        }))}
        keyed={(entry) => entry.key}
      >
        {(__rrItem0) => (
          <>
            <GalleryCase label={`file finder · lab · walked ${__rrItem0().value}`} width={640}>
              <Stand
                link={{ kind: "live", city: "sprawling" }}
                unread={[]}
                waiting={[]}
                answers={finding(__rrItem0().value)}
                children={() => (
                  <>
                    <div class="flex flex-col gap-snug rounded-panel bg-raised p-snug">
                      <Search
                        under={LAB}
                        titleId={`gd-finder-${__rrItem0().value}`}
                        onClose={() => undefined}
                      />
                    </div>
                  </>
                )}
              />
            </GalleryCase>
          </>
        )}
      </For>
      <GalleryCase label="tool lines · failed: the city's refusal, a tool's long first line cut, and no reason">
        <Stand
          link={{ kind: "live", city: "sprawling" }}
          unread={[]}
          waiting={[]}
          children={() => (
            <>
              <div data-thread class="flex flex-col">
                <For
                  each={Array.from(FAILED ?? []).map((__rrItem2, __rrIndex3) => ({
                    value: __rrItem2,
                    key: __rrItem2.at,
                  }))}
                  keyed={(entry) => entry.key}
                >
                  {(__rrItem2) => (
                    <>
                      <CallLine call={__rrItem2().value} run={RUN} />
                    </>
                  )}
                </For>
              </div>
            </>
          )}
        />
      </GalleryCase>
      <GalleryCase label="tool lines · failed, at a phone's width" width={360}>
        <Stand
          link={{ kind: "live", city: "sprawling" }}
          unread={[]}
          waiting={[]}
          children={() => (
            <>
              <div data-thread class="flex flex-col">
                <For
                  each={Array.from(FAILED ?? []).map((__rrItem4, __rrIndex5) => ({
                    value: __rrItem4,
                    key: __rrItem4.at,
                  }))}
                  keyed={(entry) => entry.key}
                >
                  {(__rrItem4) => (
                    <>
                      <CallLine call={__rrItem4().value} run={RUN} />
                    </>
                  )}
                </For>
              </div>
            </>
          )}
        />
      </GalleryCase>
      <GalleryCase label="tool lines · keys at the row's end">
        <Stand
          link={{ kind: "live", city: "sprawling" }}
          unread={[]}
          waiting={[]}
          children={() => (
            <>
              <div data-thread class="flex flex-col [&_[data-line-keys]]:inline-flex">
                <For
                  each={Array.from(CALLS ?? []).map((__rrItem6, __rrIndex7) => ({
                    value: __rrItem6,
                    key: __rrItem6.at,
                  }))}
                  keyed={(entry) => entry.key}
                >
                  {(__rrItem6) => (
                    <>
                      <CallLine call={__rrItem6().value} run={RUN} />
                    </>
                  )}
                </For>
              </div>
            </>
          )}
        />
      </GalleryCase>
    </>
  );
}
