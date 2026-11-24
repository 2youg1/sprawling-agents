// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import {
  createEffect,
  createMemo,
  createSignal,
  createUniqueId,
  For,
  onCleanup,
  Show,
} from "solid-js";

import { QUERIES } from "../../../core/asking";
import { privacyOperation } from "../../../core/commands";
import { fill, say, type Key } from "../../../core/lang";
import { storeValue } from "../../../core/solid_store";
import { ui } from "../../../ui";
import type { PrivacyAnswer, PrivacyControl } from "../../../wire";
import Button from "../../parts/button.solid";
import Dialog from "../../parts/dialog_view";
import Skeleton from "../../parts/skeleton.solid";
import Entry, { Facts } from "./entry_view";
import { confirmOf, entryId, hostFacts, sectionWord, sectionsOf, standingOf } from "./page";
import { privacyPage, type Source } from "./state";
import { controlWord, originalReason, PAGE_FACTS, reasonWord } from "./words";

export default function Privacy(props: { readonly fixture?: PrivacyAnswer | undefined }) {
  const u = ui();
  const lang = storeValue(u.lang);
  const uid = createUniqueId();
  const asked = storeValue(() =>
    props.fixture === undefined ? u.conn.asking.ask(QUERIES.privacy) : undefined,
  );
  const answer = () => {
    const frame = asked();
    return props.fixture ?? (frame !== undefined && "privacy" in frame ? frame.privacy : undefined);
  };
  const source: Source = {
    answer,
    send: (action) => {
      const { command, idem } = privacyOperation(action);
      return { idem, sent: props.fixture === undefined ? u.send(command) : true };
    },
    refresh: () => {
      if (props.fixture === undefined) u.conn.asking.refresh(QUERIES.privacy);
    },
  };
  const page = privacyPage(source);
  const held = storeValue(page.state);
  onCleanup(page.close);
  createEffect(answer, (current) => {
    page.receive(current);
  });
  const sections = createMemo(() => {
    const current = answer();
    return current === undefined ? [] : sectionsOf(current);
  });
  const models = createMemo(() => sections().flatMap((section) => section.entries));
  const standing = () => {
    const current = answer();
    return current === undefined ? "open" : standingOf(current.history);
  };
  const restorable = createMemo(() =>
    models().filter((model) => model.offers.some((offer) => offer.action === "restore")),
  );
  const confirming = createMemo(() =>
    models().find((model) => page.heldOf(model.row.control, held()).stage.kind === "confirming"),
  );
  const confirm = createMemo(() => {
    const model = confirming();
    if (model === undefined) return undefined;
    const stage = page.heldOf(model.row.control, held()).stage;
    return stage.kind === "confirming" ? confirmOf(lang(), model, stage.action) : undefined;
  });
  const [first, ...rest] = PAGE_FACTS;
  const [restoring, setRestoring] = createSignal(false);
  const word = (key: Key) => say(lang(), key);

  function reach(control: PrivacyControl): void {
    const node = document.getElementById(entryId(uid, control));
    node?.scrollIntoView({ block: "start" });
    node?.focus();
  }

  function FactsOfHost(host: { readonly answer: PrivacyAnswer }) {
    const facts = () => hostFacts(lang(), host.answer.host);
    const table = () => {
      const read = facts();
      return typeof read === "string" ? undefined : read;
    };
    const description = () => {
      const read = facts();
      return typeof read === "string" ? read : "";
    };
    return (
      <div class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
        <span class="text-label font-label text-text">{word("privacy_host_title")}</span>
        <Show when={table()} fallback={<p class="text-note text-text-quiet">{description()}</p>}>
          {(values) => <Facts values={values()} />}
        </Show>
      </div>
    );
  }

  return (
    <>
      <div class="flex min-w-0 flex-col gap-wide">
        <section aria-label={word("privacy_host_title")} class="flex max-w-talk flex-col gap-snug">
          <Show when={first} keyed>
            {(label) => <p class="text-body text-text">{word(label)}</p>}
          </Show>
          <Show when={answer()}>{(reading) => <FactsOfHost answer={reading()} />}</Show>
          <ul class="flex list-disc flex-col gap-tight pl-base text-note text-text-quiet">
            <For each={rest} keyed={true}>
              {(fact) => <li>{word(fact)}</li>}
            </For>
          </ul>
          <Show when={standing() !== "open"}>
            <p class="text-note text-alert">
              {word(
                standing() === "withheld"
                  ? "privacy_history_withheld"
                  : standing() === "unreadable"
                    ? "privacy_history_unreadable"
                    : "privacy_unresolved",
              )}
            </p>
          </Show>
          <div class="flex flex-wrap items-center gap-snug">
            <Button
              label={word("privacy_refresh")}
              tone="quiet"
              glyph="refresh"
              onPress={source.refresh}
            />
            <Show when={restorable().length > 0}>
              <Button
                label={word("privacy_restore_all")}
                tone="secondary"
                glyph="refresh"
                onPress={() => setRestoring(true)}
              />
            </Show>
          </div>
        </section>
        <Show when={answer()} fallback={<Skeleton label={word("privacy_reading")} rows={4} />}>
          {(reading) => (
            <>
              <For each={sections()} keyed={(section) => section.category}>
                {(section) => {
                  const id = () => `${uid}-section-${section().category}`;
                  return (
                    <section aria-labelledby={id()} class="flex min-w-0 flex-col gap-base">
                      <h3 id={id()} class="text-heading font-heading text-text">
                        {word(sectionWord(section().category))}
                      </h3>
                      <div class="grid grid-fit items-start gap-base">
                        <For each={section().entries} keyed={(model) => model.row.control}>
                          {(model) => (
                            <Entry
                              model={model()}
                              root={uid}
                              held={page.heldOf(model().row.control, held())}
                              onPress={(action) => page.press(model(), action)}
                            />
                          )}
                        </For>
                      </div>
                    </section>
                  );
                }}
              </For>

              <Show when={reading().not_written.length > 0}>
                <section
                  aria-labelledby={`${uid}-not-written`}
                  class="flex min-w-0 flex-col gap-base"
                >
                  <h3 id={`${uid}-not-written`} class="text-heading font-heading text-text">
                    {word("privacy_not_written_title")}
                  </h3>
                  <p class="text-note text-text-quiet">{word("privacy_not_written_hint")}</p>
                  <div class="grid grid-fit items-start gap-base">
                    <For each={reading().not_written} keyed={(entry) => entry.line.item}>
                      {(entry) => (
                        <div class="flex min-w-0 flex-col gap-tight rounded-card bg-raised px-base py-snug text-note">
                          <span class="font-mono text-text [overflow-wrap:anywhere]">
                            {entry().line.text}
                          </span>
                          <span class="text-label font-label text-text-quiet">
                            {word(reasonWord(entry().reason))}
                          </span>
                          <Show when={originalReason(entry().line.item)} keyed>
                            {(why) => <p class="text-text">{word(why)}</p>}
                          </Show>
                          <div class="flex flex-wrap items-center gap-snug">
                            <For
                              each={entry().alternatives}
                              keyed={true}
                              fallback={
                                <span class="text-text-faint">{word("privacy_none_close")}</span>
                              }
                            >
                              {(control) => (
                                <Button
                                  label={fill(word("privacy_see"), {
                                    title: word(controlWord(control, "title")),
                                  })}
                                  tone="quiet"
                                  glyph="search"
                                  onPress={() => reach(control)}
                                />
                              )}
                            </For>
                          </div>
                        </div>
                      )}
                    </For>
                  </div>
                </section>
              </Show>
            </>
          )}
        </Show>
      </div>
      <Dialog
        open={confirm() !== undefined}
        title={confirm()?.title ?? ""}
        confirmLabel={confirm()?.confirmLabel ?? ""}
        cancelLabel={word("privacy_cancel")}
        onConfirm={() => {
          const model = confirming();
          if (model !== undefined) page.confirm(model);
        }}
        onCancel={() => {
          const model = confirming();
          if (model !== undefined) page.cancel(model);
        }}
      >
        <Show when={confirm()}>
          {(look) => (
            <>
              <Facts values={look().facts} />
              <For each={look().notes} keyed={(note) => note.label}>
                {(note) => (
                  <div class="flex flex-col text-note">
                    <span class="text-text-quiet">{note().label}</span>
                    <p class="text-text">{note().text}</p>
                  </div>
                )}
              </For>
            </>
          )}
        </Show>
      </Dialog>
      <Dialog
        open={restoring()}
        title={fill(word("privacy_restore_all_title"), { count: String(restorable().length) })}
        detail={word("privacy_restore_all_detail")}
        confirmLabel={word("privacy_restore")}
        cancelLabel={word("privacy_cancel")}
        onConfirm={() => {
          setRestoring(false);
          page.restoreAll(restorable());
        }}
        onCancel={() => setRestoring(false)}
      />
    </>
  );
}
