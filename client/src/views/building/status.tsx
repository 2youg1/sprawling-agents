// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What this building has written since its last checkpoint, and where
// its branch stands. The checkpoint itself is a row naming the run, the
// room, the model and what that run cost, and it links into the run
// page - which is the way back from a line of code to the session that
// wrote it.

import { For, Show, createMemo } from "solid-js";

import { roomOf, toFragment } from "../../core/route";
import { usd } from "../../core/time";
import type { Address, CommitAnswer, FileChange, GitStatusAnswer } from "../../wire";
import { useSay, useUi } from "../../ui";

function Checkpoint(props: { readonly commit: CommitAnswer }) {
  const say = useSay();
  return (
    <p class="mb-base flex flex-wrap items-baseline gap-base text-note">
      <span class="text-text-quiet">{say("git_checkpoint")}</span>
      <a
        href={toFragment({ kind: "run", run: props.commit.run })}
        class="font-mono text-text-faint hover:text-text-quiet"
      >
        {props.commit.oid.slice(0, 7)}
      </a>
      <a
        href={toFragment({ kind: "talk", address: props.commit.actor })}
        class="text-text-faint hover:text-text-quiet"
      >
        {roomOf(props.commit.actor)}
      </a>
      <Show when={props.commit.model !== ""}>
        <span class="text-text-disabled">{props.commit.model}</span>
      </Show>
      <Show when={props.commit.spent > 0}>
        <span class="text-text-disabled">{say("commits_spent", { usd: usd(props.commit.spent) })}</span>
      </Show>
    </p>
  );
}

function Standing(props: { readonly status: GitStatusAnswer }) {
  const say = useSay();
  return (
    <p class="mb-base flex flex-wrap items-baseline gap-base text-note">
      <span class="text-text-quiet">{say("git_branch")}</span>
      <span class="font-mono text-text-faint">{props.status.branch ?? say("git_detached")}</span>
      <span class="text-text-disabled">
        {props.status.drift
          ? say("git_drift", {
              ahead: String(props.status.drift.ahead),
              behind: String(props.status.drift.behind),
            })
          : say("git_no_upstream")}
      </span>
    </p>
  );
}

export function Status(props: { readonly building: Address }) {
  const ui = useUi();
  const say = useSay();
  const asked = createMemo(() => ui.conn.asking.ask({ git_status: { building: props.building } }));
  const held = createMemo(() => {
    const answer = asked()();
    if (answer === undefined) return undefined;
    return "git_status" in answer ? answer.git_status : null;
  });
  const how = (file: FileChange) =>
    typeof file.how === "string" ? say(`change_${file.how}`) : say("change_renamed", { from: file.how.renamed.from });
  return (
    <div>
      <h2 class="mb-base text-heading font-heading">{say("bld_changes")}</h2>
      <Show when={held() !== undefined} fallback={<p class="text-text-disabled">…</p>}>
        <Show when={held()} fallback={<p class="text-text-faint">{say("git_unavailable")}</p>}>
          {(status) => (
            <>
              <Standing status={status()} />
              <Show when={status().checkpoint}>{(commit) => <Checkpoint commit={commit()} />}</Show>
              <Show
                when={status().files.length > 0}
                fallback={<p class="text-text-faint">{say("git_clean")}</p>}
              >
                <ul class="text-note">
                  <For each={status().files}>
                    {(file) => (
                      <li class="flex items-center gap-base border-b border-g1 py-snug">
                        <span class="w-figure shrink-0 text-text-faint">{how(file)}</span>
                        <span class="flex-1 truncate font-mono text-text-quiet">{file.path}</span>
                        <span class="shrink-0 font-mono text-text-disabled">
                          {typeof file.lines === "string"
                            ? say("change_binary")
                            : `+${String(file.lines.counted.added)} −${String(file.lines.counted.removed)}`}
                        </span>
                      </li>
                    )}
                  </For>
                </ul>
              </Show>
            </>
          )}
        </Show>
      </Show>
    </div>
  );
}
