// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The commits a building made, newest first, and the way back from a
// line of code to the session that wrote it. A row names the resident
// on the left and the session on the right; the session is a link to
// that room's conversation and nothing more (D52: from a diff to a
// session is a jump, not a new verb). Opening a row lists the files it
// changed against the commit before it; opening a file shows the
// patch. Scrolling to the end asks for the next page.

import { For, Show, createEffect, createMemo, createSignal, onCleanup, onMount } from "solid-js";

import { commitsQuery } from "../../core/asking";
import { toFragment } from "../../core/route";
import type { Address, CommitAnswer, CommitsAnswer, Seq } from "../../wire";
import { useSay, useUi } from "../../ui";
import { Changes } from "../changes";

function short(oid: string): string {
  return oid.slice(0, 7);
}

function Lineage(props: { readonly commit: CommitAnswer }) {
  const say = useSay();
  return (
    <Show when={props.commit.lineage.length > 1}>
      <p class="flex flex-wrap items-baseline gap-snug pb-snug text-note text-text-faint">
        <span>{say("commits_lineage")}</span>
        <For each={props.commit.lineage.slice(1)}>
          {(run) => (
            <a href={toFragment({ kind: "run", run })} class="font-mono text-text-disabled hover:text-text-quiet">
              {run.slice(0, 8)}
            </a>
          )}
        </For>
      </p>
    </Show>
  );
}

function Row(props: {
  readonly commit: CommitAnswer;
  readonly older: CommitAnswer | null;
  readonly last: boolean;
  readonly open: boolean;
  readonly onToggle: () => void;
}) {
  const say = useSay();
  const effort = () => (props.commit.effort === null || props.commit.effort === undefined ? "" : ` · ${props.commit.effort}`);
  return (
    <li class="border-b border-g1">
      <div class="flex items-center gap-base py-snug text-note">
        <button
          type="button"
          class="flex min-w-0 flex-1 items-center gap-base text-left hover:text-text"
          aria-expanded={props.open}
          onClick={() => {
            props.onToggle();
          }}
        >
          <span class={`w-base shrink-0 text-center text-text-disabled transition-transform ${props.open ? "rotate-90" : ""}`} aria-hidden="true">
            ›
          </span>
          <span class="truncate font-mono text-text-quiet">{props.commit.actor}</span>
          <span class="shrink-0 font-mono text-text-faint">{short(props.commit.oid)}</span>
          <Show when={props.commit.model !== ""}>
            <span class="hidden shrink-0 truncate text-text-disabled md:inline">
              {props.commit.model}
              {effort()}
            </span>
          </Show>
          <Show when={props.commit.lineage.length > 1}>
            <span class="shrink-0 rounded-pill bg-g2 px-snug text-text-faint" title={say("commits_lineage")}>
              {say("commits_succeeded", { n: String(props.commit.lineage.length - 1) })}
            </span>
          </Show>
        </button>
        <a
          href={toFragment({ kind: "run", run: props.commit.run })}
          class="shrink-0 font-mono text-text-disabled hover:text-text-quiet"
          title={say("tree_transcript")}
        >
          #{String(props.commit.seq)}
        </a>
        <Show when={props.commit.session}>
          {(session) => (
            <a
              href={toFragment({ kind: "talk", address: props.commit.actor })}
              class="shrink-0 rounded-control bg-g1 px-snug py-tight text-label text-text-quiet hover:bg-g2 hover:text-text"
            >
              {session()} →
            </a>
          )}
        </Show>
      </div>
      <Show when={props.open}>
        <div class="pb-base pl-wide">
          <Lineage commit={props.commit} />
          <Show
            when={props.older}
            fallback={<p class="text-text-disabled">{props.last ? say("commits_first") : "…"}</p>}
          >
            {(older) => <Changes base={older().oid} head={props.commit.oid} />}
          </Show>
        </div>
      </Show>
    </li>
  );
}

export function Commits(props: { readonly building: Address }) {
  const ui = useUi();
  const say = useSay();
  // One `before` per page asked; the first page has none. A page is
  // its own question, so an older page stays valid while the newest
  // one is re-asked after every commit.
  const [befores, setBefores] = createSignal<readonly (Seq | null)[]>([null]);
  const [open, setOpen] = createSignal<string | null>(null);
  const pages = createMemo(() =>
    befores().map((before) => {
      const held = ui.conn.asking.ask(commitsQuery(props.building, before));
      return (): CommitsAnswer | undefined => {
        const answer = held();
        return answer !== undefined && "commits" in answer ? answer.commits : undefined;
      };
    }),
  );
  const rows = createMemo(() => {
    const out: CommitAnswer[] = [];
    const seen = new Set<string>();
    for (const page of pages()) {
      for (const commit of page()?.commits ?? []) {
        // The newest page can grow between asks and overlap the next.
        if (!seen.has(commit.oid)) {
          seen.add(commit.oid);
          out.push(commit);
        }
      }
    }
    return out;
  });
  const tail = () => pages().at(-1)?.();
  const more = () => tail()?.more === true;
  const loading = () => tail() === undefined;
  const grow = () => {
    const last = tail();
    if (last?.more !== true) return;
    const edge = last.commits.at(-1)?.seq;
    if (edge === undefined) return;
    setBefores((held) => (held.at(-1) === edge ? held : [...held, edge]));
  };
  const [sentinel, setSentinel] = createSignal<HTMLLIElement | null>(null);
  onMount(() => {
    const watcher = new IntersectionObserver((entries) => {
      if (entries.some((entry) => entry.isIntersecting)) grow();
    });
    createEffect(() => {
      const edge = sentinel();
      if (edge !== null) watcher.observe(edge);
      onCleanup(() => {
        if (edge !== null) watcher.unobserve(edge);
      });
    });
    onCleanup(() => {
      watcher.disconnect();
    });
  });

  return (
    <div>
      <h2 class="mb-base text-heading font-heading">{say("bld_commits")}</h2>
      <Show when={rows().length > 0 || !loading()} fallback={<p class="text-text-disabled">…</p>}>
        <Show when={rows().length > 0} fallback={<p class="text-text-faint">{say("commits_empty")}</p>}>
          <ul>
            <For each={rows()}>
              {(commit, index) => (
                <Row
                  commit={commit}
                  older={rows()[index() + 1] ?? null}
                  last={!more() && index() === rows().length - 1}
                  open={open() === commit.oid}
                  onToggle={() => {
                    setOpen((held) => (held === commit.oid ? null : commit.oid));
                    // The oldest row on the page diffs against the next
                    // page, so opening it asks for one.
                    if (index() === rows().length - 1) grow();
                  }}
                />
              )}
            </For>
            <li ref={setSentinel} class="h-hair" aria-hidden="true" />
          </ul>
        </Show>
      </Show>
      <Show when={more()}>
        <p class="py-base text-center text-text-disabled">…</p>
      </Show>
    </div>
  );
}
