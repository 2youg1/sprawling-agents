// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, For, Show } from "solid-js";
import { QUERIES } from "../../core/asking";
import { approve } from "../../core/commands";
import { fill, say } from "../../core/lang";
import { toFragment } from "../../core/route";
import { storeValue } from "../../core/solid_store";
import { ago } from "../../core/time";
import { ui } from "../../ui";
import type { ApprovalItem } from "../../wire";
import { DiagnosticView } from "../diagnostic_view";
import Badge from "../parts/badge.solid";
import Button from "../parts/button.solid";
import Decide from "../parts/decide_view";
import Glyph from "../parts/glyph.solid";
import {
  attemptsFor,
  keepAttempt,
  seenApprovals,
  stillOpen,
  type Attempt,
} from "./approval_attempt";
import Asked from "./asked_view";
import { grouped, type Group } from "./waiting";

interface WaitingProps {
  readonly items?: readonly ApprovalItem[] | undefined;
}
const RECEIPT_MS = 10_000;
const BIN = toFragment({ kind: "record", lens: "bin" });

function Approval(props: { readonly group: Group }) {
  const u = ui();
  const lang = storeValue(u.lang);
  const attempts = attemptsFor(u);
  const values = storeValue(attempts);
  const connection = storeValue(u.conn.state);
  const receipts = storeValue(u.conn.receipts);
  const current = createMemo(() => values().get(props.group.key));
  const blocked = createMemo(() => {
    const held = current();
    if (held?.kind === "pending") return say(lang(), "wait_receipt_pending");
    if (held?.kind === "unknown") return say(lang(), "wait_receipt_unknown");
    return undefined;
  });
  const refusal = createMemo(() => {
    const held = current();
    return held?.kind === "refused" ? held.error : null;
  });
  function save(attempt: Attempt): void {
    keepAttempt(attempts, props.group.key, attempt);
  }
  function rule(verdict: "allow" | "deny"): void {
    if (blocked() !== undefined) return;
    const commands = props.group.items.flatMap((item) => {
      const command = approve(item.id, verdict);
      return "approve" in command ? [command] : [];
    });
    const sent = commands.filter((command) => u.send(command));
    if (sent.length > 0) save({ kind: "pending", commands: sent, at: u.now() });
  }
  function recheck(): void {
    const state = current();
    if (state?.kind !== "unknown" || connection().kind !== "live") return;
    const commands = stillOpen(state, props.group.items);
    if (commands.length > 0 && commands.every((command) => u.conn.command(command)))
      save({ kind: "pending", commands, at: u.now() });
  }
  createEffect(
    () => ({ attempt: current(), receipts: receipts(), live: connection().kind === "live" }),
    ({ attempt, receipts, live }) => {
      if (attempt === undefined || attempt.kind === "refused") return;
      for (const command of attempt.commands) {
        const receipt = receipts.get(command.approve.idem);
        if (receipt?.kind !== "refused") continue;
        save({ kind: "refused", commands: attempt.commands, error: receipt.error });
        return;
      }
      if (attempt.kind === "pending" && !live) save({ ...attempt, kind: "unknown" });
    },
  );
  createEffect(current, (attempt) => {
    if (attempt?.kind !== "pending") return;
    const timer = setTimeout(
      () => {
        if (current() === attempt) save({ ...attempt, kind: "unknown" });
      },
      Math.max(0, RECEIPT_MS - (u.now() - attempt.at)),
    );
    return () => {
      clearTimeout(timer);
    };
  });

  return (
    <Decide
      kind="question"
      asker={fill(say(lang(), "wait_from"), { actor: props.group.first.actor })}
      at={ago(lang(), props.group.first.created, u.now())}
      choices={[
        {
          answer: "yes",
          label: say(lang(), "wait_allow"),
          why: blocked(),
          onPress: () => rule("allow"),
        },
        {
          answer: "no",
          label: say(lang(), "wait_deny"),
          why: blocked(),
          onPress: () => rule("deny"),
        },
      ]}
    >
      <p class="text-body leading-read">{props.group.first.action_desc}</p>
      <Asked locator={props.group.first.artifact} />
      <p class="mt-snug text-note text-text-faint">
        {say(lang(), "wait_deny_keeps")}
        <a
          class="underline transition-colors ease-leave hover:text-text hover:ease-arrive"
          href={BIN}
        >
          {say(lang(), "wait_bin")}
        </a>
      </p>
      <Show when={props.group.items.length > 1 || props.group.first.tainted}>
        <p class="mt-snug flex flex-wrap items-center gap-snug text-note">
          <Show when={props.group.items.length > 1}>
            <span class="text-text-faint">
              {fill(say(lang(), "wait_same"), { n: String(props.group.items.length) })}
            </span>
          </Show>
          <Show when={props.group.first.tainted}>
            <Badge text={say(lang(), "wait_tainted")} weight="quiet" />
          </Show>
        </p>
      </Show>
      <Show when={blocked()}>
        {(words) => (
          <div
            role="status"
            class="mt-snug flex items-center gap-snug text-note text-text-quiet"
            data-control-state={current()?.kind}
          >
            <Glyph name={current()?.kind === "unknown" ? "ring" : "pulse"} size="sm" />
            {words()}
            <Show when={current()?.kind === "unknown"}>
              <Button tone="quiet" label={say(lang(), "talk_recheck_receipt")} onPress={recheck} />
            </Show>
          </div>
        )}
      </Show>
      <Show when={refusal()}>
        {(error) => (
          <div role="alert" class="mt-snug flex flex-col gap-tight text-note">
            <p class="flex items-center gap-tight text-alert">
              <Glyph name="cross" size="sm" />
              {error().code} · {error().subject}
            </p>
            <p>{error().recovery}</p>
            <DiagnosticView error={error()} lang={lang()} />
          </div>
        )}
      </Show>
    </Decide>
  );
}

export default function Waiting(props: WaitingProps) {
  const u = ui();
  const queue = storeValue(u.conn.asking.ask(QUERIES.approvals));
  const confirmed = createMemo(() => {
    const answer = queue();
    return answer !== undefined && "approvals" in answer ? answer.approvals.items : undefined;
  });
  const cards = createMemo(() => grouped(props.items ?? confirmed() ?? []));
  createEffect(confirmed, (items) => {
    if (items !== undefined) seenApprovals(attemptsFor(u), items);
  });
  return (
    <Show when={cards().length > 0}>
      <div class="my-pane flex flex-col gap-pane">
        <For each={cards()} keyed={(group) => group.key}>
          {(group) => <Approval group={group()} />}
        </For>
      </div>
    </Show>
  );
}
