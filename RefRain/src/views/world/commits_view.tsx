// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal, For, Match, Show, Switch } from "solid-js";

import { readAnswer } from "../../core/answered";
import { fill, say } from "../../core/lang";
import { MAYOR, buildingOf, roomOf, toFragment } from "../../core/route";
import { storeValue } from "../../core/solid_store";
import { scopedUi, ui } from "../../ui";
import type { Address, CommitAnswer, GitOid } from "../../wire";
import { openChanges } from "../inspect/open";
import Button from "../parts/button.solid";
import { phaseOf } from "../runs/lineage";
import { PHASE_FILL } from "../runs/phase";
import { commitStateOf, commitsIn, pickCommit, pickedCommit, sessionRun } from "./chosen";
import { HEIGHT, rowOf, widthOf, type CommitsLook } from "./commits";
import { graphOf } from "./lanes";

export interface CommitsProps {
  readonly here: Address;
  readonly inspectOnly?: boolean | undefined;
}

export function Commits(props: CommitsProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const scope = scopedUi();
  const selection = commitStateOf(scope);
  const sharedPick = storeValue(selection);
  const [selected, setSelected] = createSignal<GitOid>();
  const asked = createMemo(() => commitsIn(props.here));
  const answer = storeValue(() => u.conn.asking.ask(asked()));
  const read = createMemo(() =>
    readAnswer(answer(), (held) => ("commits" in held ? held.commits : undefined)),
  );
  const commits = createMemo(() => {
    const records = read();
    return records.kind === "held" ? records.value.commits : [];
  });
  const graph = createMemo(() => graphOf(commits()));
  const session = createMemo(() =>
    sessionRun(belief(), props.here, props.inspectOnly ? null : sharedPick()),
  );
  const picked = createMemo(() =>
    props.inspectOnly ? selected() : pickedCommit(props.here, sharedPick()),
  );

  function pick(commit: CommitAnswer): void {
    if (props.inspectOnly) setSelected(commit.oid);
    else pickCommit(commit, selection);
    const parent = commit.parents?.[0];
    if (parent !== undefined && scope === undefined)
      openChanges({ base: parent, head: commit.oid });
    if (!props.inspectOnly && commit.actor !== props.here)
      u.go({ kind: "talk", address: commit.actor });
  }

  const look = createMemo<CommitsLook>(() => {
    const current = read();
    const rows = commits();
    const here = props.here;
    return {
      width: widthOf(graph()),
      height: HEIGHT,
      rows: rows.map((commit, index) => {
        const doing = belief().runs[commit.run]?.doing;
        return rowOf({
          commit,
          graph: graph(),
          row: index,
          picked: commit.oid === picked(),
          here: commit.actor === here,
          phase: doing === undefined ? "done" : phaseOf(doing),
          mine: (at) => rows[at]?.run === session(),
          room: roomOf(commit.actor),
          pick,
        });
      }),
      labels: {
        oid: say(lang(), "world_commit_oid"),
        b3: say(lang(), "world_commit_b3"),
        parents: say(lang(), "world_parents"),
        at: say(lang(), "world_commit_at"),
      },
      more:
        current.kind === "held" && current.value.more && here !== MAYOR
          ? {
              href: toFragment({ kind: "building", address: buildingOf(here) }),
              word: fill(say(lang(), "world_commits_more"), { building: buildingOf(here) }),
            }
          : undefined,
    };
  });
  const unavailable = createMemo(() => {
    const reason = read();
    return reason.kind === "unavailable" ? reason : undefined;
  });
  return (
    <div class="min-h-0 flex-1 overflow-y-auto">
      <Switch>
        <Match when={unavailable()}>
          {(failure) => (
            <div class="flex flex-col items-start gap-snug text-note">
              <p class="text-text-faint">
                {fill(say(lang(), "answer_unavailable"), { query: failure().query })}
              </p>
              <Show when={failure().reason}>
                {(reason) => <p class="font-mono text-text-faint wrap-anywhere">{reason()}</p>}
              </Show>
              <Button
                label={say(lang(), "link_retry")}
                tone="quiet"
                onPress={() => u.conn.asking.refresh(asked())}
              />
            </div>
          )}
        </Match>
        <Match when={read().kind === "held" && commits().length === 0}>
          <p class="py-snug text-note text-text-faint">{say(lang(), "world_no_commits")}</p>
        </Match>
        <Match when={true}>
          <CommitList look={look()} />
        </Match>
      </Switch>
    </div>
  );
}

function CommitList(props: { readonly look: CommitsLook }) {
  return (
    <>
      <ol>
        <For each={props.look.rows} keyed={(row) => row.key}>
          {(row) => (
            <li>
              <button
                data-control=""
                type={row().wire.type}
                aria-expanded={row().wire["aria-expanded"] ? "true" : "false"}
                onClick={() => row().wire.onclick()}
                class={[
                  "-mr-snug grid h-bar w-full grid-cols-[auto_8ch_minmax(0,1fr)] items-center gap-x-base rounded-card pr-snug text-left text-note",
                  row().picked ? "wash-strong" : "hover:wash",
                  row().here ? "text-text" : "text-text-faint",
                ]}
              >
                <span
                  class="relative h-bar"
                  style={{ width: `${props.look.width}px` }}
                  aria-hidden="true"
                >
                  <svg
                    class="absolute inset-0 overflow-visible"
                    width={props.look.width}
                    height={props.look.height}
                    viewBox={`0 0 ${props.look.width} ${props.look.height}`}
                  >
                    <For each={row().lines}>
                      {(line) => (
                        <path
                          d={line.d}
                          fill="none"
                          stroke-width="2"
                          class={line.mine ? "stroke-accent" : "stroke-edge-input"}
                        />
                      )}
                    </For>
                  </svg>
                  <span
                    class={[
                      "absolute top-1/2 size-dot -translate-x-1/2 -translate-y-1/2 rounded-pill",
                      PHASE_FILL[row().node.phase],
                      { "ring-2 ring-accent": row().node.mine },
                    ]}
                    style={{ left: `${row().node.left}px` }}
                  />
                </span>
                <span class="figure text-text-quiet">{row().short}</span>
                <span class="truncate">
                  {row().message}
                  <span class="ml-snug text-text-faint">{row().room}</span>
                </span>
              </button>
              <Show when={row().facts}>
                {(facts) => (
                  <div class="mb-base ml-[calc(var(--spacing-base)+8ch)] border-l border-edge-panel pl-base text-note">
                    <dl class="grid grid-cols-[8ch_minmax(0,1fr)] gap-x-base text-text-quiet">
                      <dt class="text-text-faint">{props.look.labels.oid}</dt>
                      <dd class="figure truncate">{facts().oid}</dd>
                      <Show when={facts().b3}>
                        {(hash) => (
                          <>
                            <dt class="text-text-faint">{props.look.labels.b3}</dt>
                            <dd class="figure truncate">{hash()}</dd>
                          </>
                        )}
                      </Show>
                      <dt class="text-text-faint">{props.look.labels.parents}</dt>
                      <dd class="figure truncate">{facts().parents}</dd>
                      <dt class="text-text-faint">{props.look.labels.at}</dt>
                      <dd class="figure truncate">
                        <time datetime={facts().at.iso}>{facts().at.text}</time>
                      </dd>
                    </dl>
                  </div>
                )}
              </Show>
            </li>
          )}
        </For>
      </ol>
      <Show when={props.look.more}>
        {(more) => (
          <a class="block py-snug text-note text-accent hover:text-accent-hover" href={more().href}>
            {more().word}
          </a>
        )}
      </Show>
    </>
  );
}
