// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Option, Schema } from "effect";
import { createEffect, createMemo, createSignal, For, Match, Show, Switch } from "solid-js";
import { readAnswer, type Answered } from "../../core/answered";
import { commitsQuery } from "../../core/asking";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { derivedList } from "../../core/store";
import { ui } from "../../ui";
import {
  GitOid,
  type Address,
  type Answer,
  type CommitAnswer,
  type CommitsAnswer,
  type Seq,
} from "../../wire";
import EmptyState from "../parts/empty.solid";
import Field from "../parts/field.solid";
import Unanswered from "../parts/unanswered.solid";
import Commit from "./commit.solid";
import Whose from "./whose.solid";

interface Pages {
  readonly rows: readonly CommitAnswer[];
  readonly more: boolean;
  readonly newest: Answered<CommitsAnswer>;
  readonly edge: Seq | undefined;
}

function foldPages(answers: readonly (Answer | undefined)[]): Pages {
  const rows: CommitAnswer[] = [];
  const seen = new Set<GitOid>();
  for (const answer of answers) {
    if (answer === undefined || !("commits" in answer)) continue;
    for (const commit of answer.commits.commits) {
      if (!seen.has(commit.oid)) {
        seen.add(commit.oid);
        rows.push(commit);
      }
    }
  }
  const newest = readAnswer(answers.at(-1), (held) =>
    "commits" in held ? held.commits : undefined,
  );
  const tail = newest.kind === "held" ? newest.value : undefined;
  return { rows, more: tail?.more === true, newest, edge: tail?.commits.at(-1)?.seq };
}

const FIRST = foldPages([]);

export default function Commits(props: { readonly building: Address }) {
  const u = ui();
  const lang = storeValue(u.lang);
  const [befores, setBefores] = createSignal<readonly (Seq | null)[]>([null]);
  const [open, setOpen] = createSignal<string | null>(null);
  const [typed, setTyped] = createSignal("");
  const [whose, setWhose] = createSignal<GitOid | null>(null);
  const [refused, setRefused] = createSignal(false);
  const [sentinel, setSentinel] = createSignal<HTMLLIElement>();

  const asked = storeValue(() =>
    derivedList(
      befores().map((before) => u.conn.asking.ask(commitsQuery(props.building, before))),
      foldPages,
    ),
  );
  const pages = () => asked() ?? FIRST;
  const unavailable = () => {
    const answer = pages().newest;
    return answer.kind === "unavailable" ? answer : undefined;
  };
  const typedOid = createMemo(() => Option.getOrNull(Schema.decodeOption(GitOid)(typed().trim())));

  function grow(): void {
    const { more, edge } = pages();
    if (more && edge !== undefined)
      setBefores((known) => (known.at(-1) === edge ? known : [...known, edge]));
  }

  createEffect(sentinel, (node) => {
    if (node === undefined) return;
    const watcher = new IntersectionObserver((entries) => {
      if (entries.some((entry) => entry.isIntersecting)) grow();
    });
    watcher.observe(node);
    return () => watcher.disconnect();
  });

  function openRow(oid: string): void {
    setOpen(oid);
    requestAnimationFrame(() =>
      document.getElementById(`commit-${oid}`)?.scrollIntoView({ block: "nearest" }),
    );
  }

  return (
    <div class="flex min-w-0 flex-col gap-base">
      <div class="flex min-w-0 flex-wrap items-end justify-between gap-x-wide gap-y-snug">
        <h2 class="sr-only">{say(lang(), "bld_commits")}</h2>
        <form
          class="w-[42ch] max-w-full min-w-0"
          onSubmit={(event) => {
            event.preventDefault();
            const oid = typedOid();
            setRefused(oid === null);
            setWhose(oid);
          }}
        >
          <Field
            label={say(lang(), "whose_label")}
            placeholder={say(lang(), "whose_placeholder")}
            mono
            value={typed()}
            onInput={(value) => {
              setTyped(value);
              setRefused(false);
            }}
            {...(refused() ? { error: say(lang(), "whose_not_oid") } : {})}
          />
        </form>
      </div>
      <Show when={whose()}>
        {(oid) => (
          <div class="border-l border-edge-input pl-base">
            <Whose oid={oid()} />
          </div>
        )}
      </Show>
      <Switch>
        <Match when={pages().rows.length > 0}>
          <ul class="border-t border-edge" aria-label={say(lang(), "bld_commits")}>
            <For each={pages().rows} keyed={(commit) => commit.oid}>
              {(commit, index) => {
                const end = () => index() === pages().rows.length - 1;
                return (
                  <Commit
                    building={props.building}
                    commit={commit()}
                    older={pages().rows[index() + 1] ?? null}
                    first={!pages().more && end()}
                    open={open() === commit().oid}
                    onToggle={() => {
                      setOpen(open() === commit().oid ? null : commit().oid);
                      if (end()) grow();
                    }}
                    holds={(oid) => pages().rows.some((row) => row.oid === oid)}
                    onOpen={openRow}
                  />
                );
              }}
            </For>
            <li
              ref={(node) => {
                setSentinel(node);
              }}
              class="h-hair"
              aria-hidden="true"
            />
          </ul>
          <p class="text-note text-text-faint">
            {say(lang(), pages().more ? "commits_more" : "commits_all")}
          </p>
        </Match>
        <Match when={pages().newest.kind === "held"}>
          <EmptyState missing="commits_empty" />
        </Match>
        <Match when={pages().newest.kind === "asking"}>
          <p class="text-text-faint">…</p>
        </Match>
      </Switch>
      <Show when={unavailable()}>
        {(answer) => (
          <Unanswered
            query={answer().query}
            asked={commitsQuery(props.building, befores().at(-1) ?? null)}
          />
        )}
      </Show>
    </div>
  );
}
