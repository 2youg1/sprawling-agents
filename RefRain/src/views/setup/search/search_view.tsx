// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, For, Show } from "solid-js";

import { say } from "../../../core/lang";
import { storeValue } from "../../../core/solid_store";
import { ui } from "../../../ui";
import type { IdemKey, SettledSearch } from "../../../wire";
import { DiagnosticView } from "../../diagnostic_view";
import Button from "../../parts/button.solid";
import Field from "../../parts/field.solid";
import Segmented from "../../parts/segmented.solid";
import { RECEIPT_MS } from "../../settings/saving";
import { HALL } from "../../shared/buildings";
import Accounts from "../providers/accounts.solid";
import { lookOf, type SearchLook } from "./search";
import { freshSearch, refuse, settle, type SearchHands } from "./search_editor";

function reason(why: string | undefined): { why?: string } {
  return why === undefined ? {} : { why };
}

export interface SearchCardProps {
  readonly settled?: SettledSearch | undefined;
}

function SearchForm(props: {
  readonly look: SearchLook;
  readonly changed: () => void;
  readonly pending: boolean;
}) {
  const look = () => props.look;
  return (
    <>
      <div class="flex flex-col gap-hair">
        <h3 class="text-label font-label text-text">{look().heading}</h3>
        <p class="text-note text-text-faint">{look().intro}</p>
      </div>
      <Segmented
        label={look().choice.label}
        tone="accent"
        options={look().choice.options}
        held={look().choice.held}
        onPick={(choice) => {
          look().choice.pick(choice);
          props.changed();
        }}
      />
      <p class="text-note text-text-faint">{look().defaultAt}</p>
      <Show when={look().override}>
        <p class="text-note text-text-quiet">{look().override}</p>
      </Show>
      <Show when={look().noFallback}>
        <p class="text-note text-text-quiet">{look().noFallback}</p>
      </Show>
      <p class="text-note text-text-faint">{look().confidential}</p>
      <Show when={look().suppliers.length > 0}>
        <h4 class="font-label text-text-quiet">{look().suppliersHeading}</h4>
        <ul class="flex flex-col gap-snug">
          <For each={look().suppliers} keyed={(supplier) => supplier.id}>
            {(supplier) => (
              <li class="flex min-w-0 flex-col gap-tight rounded-card bg-chrome px-base py-snug text-note">
                <div class="flex min-w-0 flex-wrap items-center gap-snug">
                  <span class={["font-mono", supplier().selected ? "text-accent" : "text-text"]}>
                    {supplier().id}
                  </span>
                  <span class="min-w-0 flex-1 truncate font-mono text-text-faint">
                    {supplier().url}
                  </span>
                  <span class="font-mono text-text-quiet">{supplier().remote}</span>
                  <span class="flex flex-wrap justify-end gap-tight">
                    <For each={supplier().controls} keyed={(entry) => entry.key}>
                      {(entry) => (
                        <Button
                          label={entry().label}
                          tone="quiet"
                          glyph={
                            entry().key === "edit"
                              ? "propose"
                              : entry().key === "remove"
                                ? "cross"
                                : "check"
                          }
                          {...reason(entry().why)}
                          onPress={() => {
                            entry().press();
                            props.changed();
                          }}
                        />
                      )}
                    </For>
                  </span>
                </div>
                <details>
                  <summary data-control="surface" class="text-text-quiet">
                    {supplier().accounts}
                  </summary>
                  <div class="pt-snug">
                    <Accounts roster={supplier().roster} />
                  </div>
                </details>
              </li>
            )}
          </For>
        </ul>
      </Show>
      <Show when={look().form}>
        {(form) => (
          <details open={form().open}>
            <summary data-control="surface" class="font-label text-text-quiet">
              {form().title}
            </summary>
            <form
              class="flex flex-col gap-snug pt-snug"
              onSubmit={(event) => {
                event.preventDefault();
                if (props.pending || form().save.why !== undefined) return;
                form().save.press();
                props.changed();
              }}
            >
              <For each={form().fields} keyed={(field) => field.name}>
                {(field) => (
                  <Field
                    label={field().label}
                    value={field().value}
                    mono
                    disabled={field().disabled}
                    onInput={(value) => {
                      field().input(value);
                      props.changed();
                    }}
                  />
                )}
              </For>
              <span class="flex flex-wrap gap-snug">
                <Button
                  type="submit"
                  tone="secondary"
                  glyph="check"
                  label={form().save.label}
                  loading={props.pending}
                  {...reason(form().save.why)}
                />
                <Show when={form().cancel}>
                  {(cancel) => (
                    <Button
                      tone="quiet"
                      glyph="cross"
                      label={cancel().label}
                      onPress={() => {
                        cancel().press();
                        props.changed();
                      }}
                    />
                  )}
                </Show>
              </span>
            </form>
          </details>
        )}
      </Show>
      <Show when={look().note}>
        <p role="alert" class="text-note text-alert">
          {look().note}
        </p>
      </Show>
    </>
  );
}

export default function SearchCard(props: SearchCardProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const query = { config: { addr: HALL } };
  const config = storeValue(u.conn.asking.ask(query));
  const receipts = storeValue(u.conn.receipts);
  const editor = freshSearch();
  const [rev, setRev] = createSignal(0);
  const [pending, setPending] = createSignal<IdemKey>();
  const [unknown, setUnknown] = createSignal(false);
  const change = (): void => {
    setRev((before) => before + 1);
  };
  const refresh = (): void => u.conn.asking.refresh(query);
  const answer = () => {
    const actual = config();
    return (
      props.settled ??
      (actual !== undefined && "config" in actual ? actual.config.search : undefined)
    );
  };

  createEffect(
    () => ({ settled: answer(), receipts: receipts(), request: pending() }),
    ({ settled, receipts, request }) => {
      if (request === undefined || settled === undefined) return;
      const reply = receipts.get(request);
      if (reply?.kind === "refused") {
        refuse(editor, reply.error);
      } else {
        settle(editor, settled);
        if (editor.sent !== null) return;
      }
      setPending(undefined);
      setUnknown(false);
      change();
    },
  );

  createEffect(pending, (operation) => {
    if (operation === undefined) return;
    const poll = window.setInterval(refresh, 1_000);
    const timer = window.setTimeout(() => {
      setUnknown(true);
      window.clearInterval(poll);
    }, RECEIPT_MS);
    return () => {
      window.clearInterval(poll);
      window.clearTimeout(timer);
    };
  });
  function hands(settled: SettledSearch): SearchHands {
    return {
      settled: () => settled,
      lang,
      send: (command) => {
        if (pending() !== undefined || !("configure_city" in command) || !u.send(command))
          return false;
        setPending(command.configure_city.idem);
        refresh();
        return true;
      },
    };
  }
  const look = createMemo(() => {
    rev();
    const held = answer();
    return held === undefined ? undefined : lookOf(editor, hands(held));
  });
  const busy = () => pending() !== undefined && !unknown();
  return (
    <Show when={look()}>
      {(drawn) => (
        <section
          class="flex min-w-0 flex-col gap-snug rounded-card bg-raised px-base py-snug"
          aria-label={drawn().heading}
        >
          <SearchForm look={drawn()} changed={change} pending={pending() !== undefined} />
          <Show when={busy()}>
            <p role="status" class="text-note text-text-quiet">
              {say(lang(), "saving_sent")}
            </p>
          </Show>
          <Show when={unknown()}>
            <p role="status" class="text-note text-text-quiet">
              {say(lang(), "saving_unverified")}
            </p>
            <Button label={say(lang(), "saving_check")} result="unknown" onPress={refresh} />
          </Show>
          <Show when={drawn().refused}>
            {(error) => <DiagnosticView error={error()} lang={lang()} />}
          </Show>
        </section>
      )}
    </Show>
  );
}
