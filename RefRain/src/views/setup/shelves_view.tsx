// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, For, Show, untrack } from "solid-js";
import { configureSkillShelves } from "../../core/commands";
import { fill, say, type Key } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Address, AxError, IdemKey, Query } from "../../wire";
import { DiagnosticView } from "../diagnostic_view";
import Button from "../parts/button.solid";
import Copy from "../parts/copy.solid";
import Field from "../parts/field.solid";
import { noticeTitle } from "../parts/notice_title";
import { RECEIPT_MS } from "../settings/saving";
import { HALL } from "../shared/buildings";
import { alreadyMounted, offered, withLibrary, withoutLibrary } from "./skill_libraries";

interface Pending {
  readonly path: string;
  readonly adding: boolean;
  readonly idem: IdemKey;
}

interface Refused {
  readonly path: string;
  readonly error: AxError;
}

export default function Shelves(props: { readonly building?: Address }) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const receipts = storeValue(u.conn.receipts);
  const city = () => belief().city ?? "<city>";
  const building = () => props.building ?? HALL;
  const query = createMemo<Query>(() => ({ skills: { building: building() } }));
  const answer = storeValue(() => u.conn.asking.ask(query()));
  const mounted = createMemo(() => {
    const held = answer();
    return held !== undefined && "skills" in held ? (held.skills.external_shelves ?? []) : null;
  });
  const [wanted, setWanted] = createSignal("");
  const [pending, setPending] = createSignal<Pending>();
  const [refused, setRefused] = createSignal<Refused>();
  const [unknown, setUnknown] = createSignal(false);
  const offer = createMemo(() => offered(wanted()));
  const word = (key: Key) => say(lang(), key);
  const refresh = () => u.conn.asking.refresh(query());

  function offerWhy(): Key | undefined {
    const choice = offer();
    if (choice.kind === "blank") return "skill_mount_needs_path";
    if (choice.kind === "refused") return choice.why;
    return alreadyMounted(mounted() ?? [], choice.path) ? "skill_mount_taken" : undefined;
  }

  createEffect(
    () => ({ mounted: mounted(), operation: pending(), receipts: receipts() }),
    ({ mounted, operation, receipts }) => {
      if (operation === undefined) return;
      const observed = receipts.get(operation.idem);
      if (observed?.kind === "refused") {
        setRefused({ path: operation.path, error: observed.error });
        setPending(undefined);
        refresh();
        return;
      }
      if (mounted === null || alreadyMounted(mounted, operation.path) !== operation.adding) return;
      setPending(undefined);
      if (operation.adding && untrack(wanted).trim() === operation.path) setWanted("");
    },
  );

  createEffect(pending, (operation) => {
    if (operation === undefined) return;
    const poll = window.setInterval(refresh, 1_000);
    const timeout = window.setTimeout(() => {
      window.clearInterval(poll);
      setUnknown(true);
    }, RECEIPT_MS);
    return () => {
      window.clearInterval(poll);
      window.clearTimeout(timeout);
    };
  });

  function issue(path: string, adding: boolean): void {
    const held = mounted();
    if (held === null || pending() !== undefined) return;
    if (adding === alreadyMounted(held, path)) return;
    const command = configureSkillShelves(
      adding ? withLibrary(held, path) : withoutLibrary(held, path),
    );
    if (!("configure_city" in command) || !u.send(command)) return;
    setRefused(undefined);
    setUnknown(false);
    setPending({ path, adding, idem: command.configure_city.idem });
    refresh();
  }

  function add(event: SubmitEvent): void {
    event.preventDefault();
    const choice = offer();
    if (choice.kind === "path" && offerWhy() === undefined) issue(choice.path, true);
  }

  const stopped = () => {
    const why = offerWhy();
    return why === undefined ? {} : { why: word(why) };
  };

  const folders = createMemo<readonly { readonly name: Key; readonly path: string }[]>(() => [
    { name: "setup_skills", path: `${city()}/.sprawling/library/` },
    { name: "setup_skills_building", path: `${city()}/${building()}/.sprawling/skills/` },
  ]);

  return (
    <div class="flex flex-col gap-snug text-note text-text-quiet">
      <For each={folders()} keyed={(folder) => folder.name}>
        {(folder) => (
          <>
            <p>{word(folder().name)}</p>
            <div class="flex min-w-0 items-center gap-snug">
              <code class="min-w-0 truncate rounded-control bg-raised px-base py-snug font-mono text-text">
                {folder().path}
              </code>
              <Copy text={folder().path} />
            </div>
          </>
        )}
      </For>
      <p class="text-text-faint">{word("setup_skills_how")}</p>
      <div class="mt-base flex min-w-0 flex-col gap-snug border-t border-edge pt-base">
        <p class="font-label text-text">{word("skill_mount_title")}</p>
        <Show
          when={mounted() !== null}
          fallback={<p class="text-text-faint">{word("skill_mount_untold")}</p>}
        >
          <Show
            when={(mounted()?.length ?? 0) > 0}
            fallback={<p class="text-text-faint">{word("skill_mount_none")}</p>}
          >
            <ul class="flex min-w-0 flex-col">
              <For each={mounted()}>
                {(path) => (
                  <li class="flex min-w-0 items-center gap-snug py-tight">
                    <code class="min-w-0 flex-1 truncate font-mono text-text">{path}</code>
                    <Button
                      label={word("skill_mount_remove")}
                      name={fill(word("skill_mount_remove_name"), { path })}
                      tone="quiet"
                      glyph="cross"
                      loading={pending()?.path === path && !unknown()}
                      onPress={() => issue(path, false)}
                    />
                  </li>
                )}
              </For>
            </ul>
          </Show>
          <form class="flex flex-wrap items-end gap-snug" onSubmit={add}>
            <div class="min-w-[min(100%,40ch)] flex-1">
              <Field
                label={word("skill_mount_label")}
                value={wanted()}
                mono
                placeholder={word("skill_mount_placeholder")}
                {...(offer().kind === "refused"
                  ? { error: word(offerWhy() ?? "skill_mount_needs_path") }
                  : {})}
                onInput={setWanted}
              />
            </div>
            <Button
              type="submit"
              tone={offerWhy() === undefined ? "primary" : "secondary"}
              label={word("skill_mount_add")}
              loading={pending()?.adding === true && !unknown()}
              {...stopped()}
            />
          </form>
          <p class="text-text-faint">{word("skill_mount_help")}</p>
          <Show when={pending()}>
            <p class="text-text-faint" role="status">
              {word(unknown() ? "saving_unverified" : "skill_mount_waiting")}
            </p>
            <Show when={unknown()}>
              <Button label={word("saving_check")} glyph="search" onPress={refresh} />
            </Show>
          </Show>
          <Show when={refused()}>
            {(line) => (
              <>
                <p class="text-note text-alert" role="alert">
                  {fill(word("skill_mount_refused"), {
                    path: line().path,
                    why: noticeTitle(lang(), line().error.code, line().error.subject),
                  })}
                </p>
                <DiagnosticView error={line().error} lang={lang()} />
              </>
            )}
          </Show>
        </Show>
      </div>
    </div>
  );
}
