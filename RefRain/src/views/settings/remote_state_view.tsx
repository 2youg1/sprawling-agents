// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createSignal, createUniqueId, For, Match, Show, Switch } from "solid-js";

import { fill, say, type Key } from "../../core/lang";
import { encode } from "../../core/remote/base32";
import { storeValue } from "../../core/solid_store";
import { isoInstant, moment } from "../../core/time";
import { ui } from "../../ui";
import type { DoctorCustodyLifetime } from "../../wire";
import Button from "../parts/button.solid";
import Copy from "../parts/copy.solid";
import Dialog from "../parts/dialog_view";
import { LACKS, STEPS, VERBS, grouped, restartOf, toldKey, type Standing } from "./remote";

interface RemoteStateProps {
  readonly standing: Standing;
  readonly keeps: DoctorCustodyLifetime | null;
  readonly onPair: () => void;
  readonly onSeen: () => void;
  readonly onLock: () => void;
  readonly onForget: () => void;
}

const LABEL = "text-note text-text-faint";
const VALUE = "font-mono text-note text-text wrap-anywhere";

export default function RemoteState(props: RemoteStateProps) {
  const lang = storeValue(ui().lang);
  const uid = createUniqueId();
  const [forgetting, setForgetting] = createSignal(false);
  let deviceHeading: HTMLElement | undefined;
  let forgetButton: HTMLElement | undefined;
  let was: Standing["kind"] = "reading";
  createEffect(
    () => props.standing.kind,
    (now) => {
      if (was === "seed" && now === "paired") requestAnimationFrame(() => deviceHeading?.focus());
      was = now;
    },
  );

  function status(): string {
    const standing = props.standing;
    if ((standing.kind !== "invited" && standing.kind !== "paired") || standing.told === null)
      return "";
    const told = toldKey(standing.told);
    return fill(say(lang(), told.key), { code: told.code ?? "" });
  }

  const word = (key: Key) => say(lang(), key);
  const invited = () => (props.standing.kind === "invited" ? props.standing : undefined);
  const shown = () => (props.standing.kind === "seed" ? props.standing : undefined);
  const paired = () => (props.standing.kind === "paired" ? props.standing : undefined);
  const absent = () =>
    props.standing.kind === "unpaired" ||
    props.standing.kind === "unreadable" ||
    props.standing.kind === "lacking";
  const lack = () =>
    props.standing.kind === "lacking"
      ? LACKS[props.standing.lack]
      : props.standing.kind === "unreadable"
        ? "remote_unreadable"
        : undefined;
  const restart = () => word(restartOf(props.keeps));

  return (
    <>
      <div class="flex flex-col gap-wide">
        <Switch>
          <Match when={absent()}>
            <p class="text-body text-text-quiet">{word("remote_about")}</p>
            <Show when={lack()}>
              {(problem) => <p class="text-body text-alert">{word(problem())}</p>}
            </Show>
            <section class="flex flex-col gap-snug" aria-labelledby={`${uid}-steps`}>
              <h3 id={`${uid}-steps`} class="text-label font-label text-text">
                {word("remote_steps")}
              </h3>
              <ol class="flex flex-col">
                <For each={STEPS} keyed={true}>
                  {(step, index) => (
                    <li class="grid grid-cols-[2rem_minmax(0,1fr)] gap-x-snug border-b border-edge py-snug">
                      <span class="font-mono text-label text-text-quiet" aria-hidden="true">
                        {String(index() + 1)}
                      </span>
                      <div class="flex min-w-0 flex-col gap-tight">
                        <p class="text-note text-text">{word(step.key)}</p>
                        <Show when={step.spelling}>
                          {(spelling) => (
                            <div class="flex min-w-0 items-start gap-tight">
                              <code class="min-w-0 flex-1 break-all rounded-control bg-chrome px-snug py-tight font-mono text-note text-text-quiet">
                                {spelling()}
                              </code>
                              <Copy text={spelling()} />
                            </div>
                          )}
                        </Show>
                      </div>
                    </li>
                  )}
                </For>
                <li class="grid grid-cols-[2rem_minmax(0,1fr)] gap-x-snug py-snug">
                  <span class="font-mono text-label text-text-quiet" aria-hidden="true">
                    {String(STEPS.length + 1)}
                  </span>
                  <p class="text-note text-text">{restart()}</p>
                </li>
              </ol>
            </section>
            <section class="flex flex-col" aria-labelledby={`${uid}-console`}>
              <h3 id={`${uid}-console`} class="pb-tight text-label font-label text-text">
                {word("remote_console")}
              </h3>
              <dl class="flex flex-col">
                <For each={VERBS} keyed={true}>
                  {([spelling, does]) => (
                    <div class="grid grid-cols-[minmax(0,16rem)_minmax(0,1fr)] gap-x-base border-b border-edge py-snug @max-measure/page:grid-cols-1">
                      <dt>
                        <code class="font-mono text-note text-text">{spelling}</code>
                      </dt>
                      <dd class="text-note text-text-quiet">{word(does)}</dd>
                    </div>
                  )}
                </For>
              </dl>
            </section>
            <Show when={props.standing.kind === "unpaired"}>
              <p class="text-note text-text-faint">{word("remote_not_paired")}</p>
            </Show>
          </Match>
          <Match when={invited()}>
            {(invite) => (
              <section class="flex flex-col gap-base" aria-labelledby={`${uid}-invited`}>
                <h3 id={`${uid}-invited`} class="text-label font-label text-text">
                  {word("remote_invited")}
                </h3>
                <p class={VALUE}>
                  {fill(word("remote_invited_city"), {
                    fingerprint: grouped(encode(invite().invitation.city).slice(0, 12)),
                  })}
                </p>
                <p class="text-body text-text-quiet">{word("remote_invited_about")}</p>
                <div>
                  <Button
                    label={word(invite().busy ? "remote_pairing" : "remote_pair")}
                    tone="primary"
                    glyph="key"
                    loading={invite().busy}
                    onPress={props.onPair}
                  />
                </div>
              </section>
            )}
          </Match>
          <Match when={shown()}>
            {(seed) => (
              <section class="flex flex-col gap-base" aria-labelledby={`${uid}-seed`}>
                <h3 id={`${uid}-seed`} class="text-label font-label text-text">
                  {word("remote_seed")}
                </h3>
                <output
                  class="asks rounded-card px-base py-snug font-mono text-heading text-text"
                  aria-label={word("remote_seed")}
                >
                  {grouped(seed().seed)}
                </output>
                <p class="text-body text-text-quiet">{word("remote_seed_about")}</p>
                <Show when={!seed().persisted}>
                  <p class="text-note text-text-faint">{word("remote_persist_refused")}</p>
                </Show>
                <div>
                  <Button
                    label={word("remote_seed_seen")}
                    tone="primary"
                    glyph="check"
                    onPress={props.onSeen}
                  />
                </div>
              </section>
            )}
          </Match>
          <Match when={paired()}>
            {(device) => (
              <section class="flex flex-col gap-base" aria-labelledby={`${uid}-device`}>
                <h3
                  id={`${uid}-device`}
                  tabindex="-1"
                  class="text-label font-label text-text"
                  ref={(node) => {
                    deviceHeading = node;
                  }}
                >
                  {word("remote_device")}
                </h3>
                <dl class="grid grid-cols-[minmax(0,8rem)_minmax(0,1fr)] gap-x-base gap-y-snug">
                  <dt class={LABEL}>{word("remote_device_city")}</dt>
                  <dd class={VALUE}>{grouped(encode(device().device.fingerprint))}</dd>
                  <dt class={LABEL}>{word("remote_device_id")}</dt>
                  <dd class={VALUE}>{encode(device().device.id)}</dd>
                  <dt class={LABEL}>{word("remote_device_at")}</dt>
                  <dd class={VALUE}>
                    <time datetime={isoInstant(device().device.at)}>
                      {moment(device().device.at)}
                    </time>
                  </dd>
                </dl>
                <p class="text-note text-text-faint">{restart()}</p>
                <div class="flex flex-wrap gap-snug">
                  <Button
                    label={word(device().busy ? "remote_locking" : "remote_lock")}
                    loading={device().busy}
                    glyph="gate"
                    onPress={props.onLock}
                  />
                  <span
                    class="contents"
                    ref={(node) => {
                      forgetButton = node;
                    }}
                  >
                    <Button
                      label={word("remote_forget")}
                      tone="destructive"
                      glyph="cross"
                      onPress={() => setForgetting(true)}
                    />
                  </span>
                </div>
              </section>
            )}
          </Match>
        </Switch>
        <p role="status" class="text-body text-text-quiet empty:hidden">
          {status()}
        </p>
      </div>
      <Dialog
        open={forgetting()}
        title={word("remote_forget_title")}
        detail={word("remote_forget_detail")}
        confirmLabel={word("remote_forget_confirm")}
        cancelLabel={word("part_cancel")}
        destructive
        onConfirm={() => {
          setForgetting(false);
          props.onForget();
        }}
        onCancel={() => {
          setForgetting(false);
          forgetButton?.querySelector("button")?.focus();
        }}
      />
    </>
  );
}
