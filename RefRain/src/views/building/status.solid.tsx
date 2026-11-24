// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, For, Match, Show, Switch } from "solid-js";
import { fill, say } from "../../core/lang";
import { roomOf, toFragment } from "../../core/route";
import { storeValue } from "../../core/solid_store";
import { usd } from "../../core/time";
import { ui } from "../../ui";
import type { Address, CommitAnswer, GitStatusAnswer } from "../../wire";
import { howWord, linesWord, shortOid } from "../changes";
import { inspectorItems, openDocument, rightItem } from "../inspect/open";
import EmptyState from "../parts/empty.solid";
import FactLink from "./fact_link.look.solid";
import { fileRowWire } from "./file_row";
import FileRow from "./file_row.look.solid";
import TakeBack from "./take_back.solid";

function Checkpoint(props: { readonly commit: CommitAnswer }) {
  const lang = storeValue(ui().lang);
  return (
    <p class="flex flex-wrap items-baseline gap-x-base text-note">
      <span class="text-text-faint">{say(lang(), "git_checkpoint")}</span>
      <FactLink
        label={shortOid(props.commit.oid)}
        face="figure"
        ink="quiet"
        wire={{ href: toFragment({ kind: "run", run: props.commit.run }) }}
      />
      <FactLink
        label={roomOf(props.commit.actor)}
        face="words"
        ink="quiet"
        wire={{ href: toFragment({ kind: "talk", address: props.commit.actor }) }}
      />
      <Show when={props.commit.model !== ""}>
        <span class="text-text-faint">{props.commit.model}</span>
      </Show>
      <Show when={props.commit.spent > 0}>
        <span class="figure text-text-faint">
          {fill(say(lang(), "commits_spent"), { usd: usd(props.commit.spent) })}
        </span>
      </Show>
    </p>
  );
}

export default function Status(props: { readonly building: Address }) {
  const u = ui();
  const lang = storeValue(u.lang);
  const inspector = storeValue(inspectorItems);
  const asked = storeValue(() => u.conn.asking.ask({ git_status: { building: props.building } }));
  const status = createMemo((): GitStatusAnswer | null | undefined => {
    const answer = asked();
    if (answer === undefined) return undefined;
    return "git_status" in answer ? answer.git_status : null;
  });
  function isOpen(path: string): boolean {
    const item = rightItem(inspector());
    return item?.kind === "document" && item.building === props.building && item.path === path;
  }

  return (
    <div class="flex min-w-0 flex-col gap-base">
      <h2 class="text-note text-text-faint">{say(lang(), "git_since_checkpoint")}</h2>
      <Switch fallback={<p class="text-text-faint">…</p>}>
        <Match when={status() === null}>
          <EmptyState missing="git_unavailable" />
        </Match>
        <Match when={status()}>
          {(held) => (
            <>
              <p class="flex flex-wrap items-baseline gap-x-base text-note">
                <span class="text-text-faint">{say(lang(), "git_branch")}</span>
                <span class="font-mono text-text-quiet">
                  {held().branch ?? say(lang(), "git_detached")}
                </span>
                <span class="text-text-faint">
                  <Show when={held().drift} fallback={say(lang(), "git_no_upstream")}>
                    {(drift) =>
                      fill(say(lang(), "git_drift"), {
                        ahead: String(drift().ahead),
                        behind: String(drift().behind),
                      })
                    }
                  </Show>
                </span>
              </p>
              <Show when={held().checkpoint}>{(commit) => <Checkpoint commit={commit()} />}</Show>
              <Show when={held().files.length > 0} fallback={<EmptyState missing="git_clean" />}>
                <ul class="border-t border-edge text-note">
                  <For each={held().files} keyed={(file) => file.path}>
                    {(file) => (
                      <li class="flex min-w-0 items-center gap-snug border-b border-edge">
                        <FileRow
                          how={howWord(lang(), file().how)}
                          path={file().path}
                          lines={linesWord(lang(), file().lines)}
                          current={isOpen(file().path)}
                          wire={fileRowWire(
                            fill(say(lang(), "git_open_file"), { path: file().path }),
                            isOpen(file().path),
                            () => {
                              openDocument({
                                building: props.building,
                                path: file().path,
                                version: null,
                              });
                            },
                          )}
                        />
                        <Show when={held().checkpoint?.oid}>
                          {(point) => (
                            <TakeBack
                              building={props.building}
                              path={file().path}
                              point={point()}
                            />
                          )}
                        </Show>
                      </li>
                    )}
                  </For>
                </ul>
              </Show>
            </>
          )}
        </Match>
      </Switch>
    </div>
  );
}
