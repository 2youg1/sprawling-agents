// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal, createUniqueId, For, Show } from "solid-js";
import { MODES, WRITE_LIMITS, changeRunPolicy, nameSession } from "../../core/commands";
import { fill, say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { PIN, given, kept, readTag, stripped } from "../../core/tags";
import { ui } from "../../ui";
import type { RunPolicy, Tag } from "../../wire";
import ControlTip from "../parts/control_tip";
import Glyph from "../parts/glyph.solid";
import { opening, popupOf, walking } from "./menu";
import type { Item } from "./session_menu";
import type { MenuProps } from "./sessions";

export default function SessionMenu(props: MenuProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const tags = storeValue(u.tags.held);
  const belief = storeValue(u.conn.belief);
  const [open, setOpen] = createSignal(false);
  const [naming, setNaming] = createSignal<"tag" | "name" | null>(null);
  const [typed, setTyped] = createSignal("");
  const uid = createUniqueId();
  const itemNodes = new Map<string, HTMLButtonElement>();
  let trigger: HTMLButtonElement | undefined;
  let field: HTMLInputElement | undefined;
  const answer = storeValue(() => {
    const run = props.session.run;
    return open() && run !== null ? u.conn.asking.ask({ rounds: { run } }) : undefined;
  });
  const policy = createMemo<RunPolicy | null>(() => {
    const name = props.named;
    if (name === null || props.session.run === null) return null;
    const changed = belief().policies[name.room] ?? null;
    if (changed !== null && changed.seq > name.began) return changed.policy;
    const report = answer();
    return report !== undefined && "rounds" in report
      ? (report.rounds.opening?.policy ?? null)
      : null;
  });
  const why = () => (props.named === null ? say(lang(), "world_tags_offline") : undefined);
  const name = () => fill(say(lang(), "world_row_menu"), { room: props.label });
  const wrong = () => naming() === "tag" && typed().trim() !== "" && readTag(typed()) === null;

  function leave(): void {
    setOpen(false);
    setNaming(null);
  }
  function close(): void {
    leave();
    requestAnimationFrame(() => trigger?.focus({ preventScroll: true }));
  }
  function live(): HTMLButtonElement[] {
    return items().flatMap((item) => itemNodes.get(item.id) ?? []);
  }
  function show(): void {
    if (props.named === null) return;
    setOpen(true);
    setNaming(null);
    setTyped("");
    requestAnimationFrame(() => live()[0]?.focus());
  }
  function edit(which: "tag" | "name", start: string): void {
    setNaming(which);
    setTyped(start);
    requestAnimationFrame(() => field?.focus());
  }
  function change(how: typeof given, tag: Tag): void {
    const named = props.named;
    if (named !== null) u.tags.retag(how(tags(), named, tag));
    close();
  }
  function repolicy(next: RunPolicy): void {
    const named = props.named;
    if (named !== null) u.conn.command(changeRunPolicy(named.room, next));
    close();
  }
  function submit(event: SubmitEvent): void {
    event.preventDefault();
    if (naming() === "name") {
      const named = props.named;
      if (named !== null) u.conn.command(nameSession(named.room, named.began, typed()));
      close();
      return;
    }
    const tag = readTag(typed());
    if (tag !== null) change(given, tag);
  }
  const items = createMemo<Item[]>(() => {
    const choices: Item[] = [];
    const reading = props.read;
    if (reading !== null)
      choices.push({
        id: "read",
        word: say(lang(), "world_read_session"),
        act: () => {
          u.go(reading);
          close();
        },
      });
    if (props.pinning !== "mayor")
      choices.push({
        id: "pin",
        word: say(lang(), props.pinning === "tagged" ? "world_unpin" : "world_pin"),
        act: () => change(props.pinning === "tagged" ? stripped : given, PIN),
      });
    const named = props.named;
    if (named !== null && kept(tags(), named))
      for (const tag of props.tags) {
        if (tag !== PIN)
          choices.push({
            id: `strip-${tag}`,
            word: fill(say(lang(), "world_tag_remove"), { tag }),
            act: () => change(stripped, tag),
          });
      }
    choices.push(
      { id: "add", word: say(lang(), "world_tag_add"), act: () => edit("tag", "") },
      {
        id: "rename",
        word: say(lang(), "world_rename"),
        act: () => edit("name", props.session.name),
      },
    );
    const now = policy();
    if (now === null) return choices;
    const mode = MODES.find((each) => each !== now.mode);
    const write = WRITE_LIMITS.find((each) => each !== now.write);
    if (mode !== undefined)
      choices.push({
        id: "mode",
        word: fill(say(lang(), "world_mode_to"), { mode: say(lang(), `mode_${mode}`) }),
        act: () => repolicy({ ...now, mode }),
      });
    if (write !== undefined)
      choices.push({
        id: "write",
        word: fill(say(lang(), "world_write_to"), {
          limit: say(lang(), `admission_value_${write}`),
        }),
        act: () => repolicy({ ...now, write }),
      });
    return choices;
  });
  const popup = `${uid}-menu`;
  const hint = `${uid}-hint`;
  const hands = { live, show, close, leave };

  return (
    <div class="relative">
      <ControlTip id={`${uid}-why`} text={why()}>
        <button
          ref={(node) => {
            trigger = node;
          }}
          type="button"
          class="grid size-control-sm place-items-center rounded-control text-text-quiet hover:wash hover:text-text aria-expanded:wash aria-expanded:text-text disabled:text-text-disabled"
          data-control=""
          aria-haspopup="menu"
          aria-expanded={open() ? "true" : "false"}
          aria-controls={popup}
          aria-label={name()}
          aria-describedby={why() === undefined ? undefined : `${uid}-why`}
          disabled={why() !== undefined}
          onClick={() => {
            if (open()) close();
            else show();
          }}
          onKeyDown={(event) => opening(open(), hands)(event)}
        >
          <Glyph name="more" size="sm" />
        </button>
      </ControlTip>
      <Show when={open()}>
        <div
          id={popup}
          onFocusOut={(event) => popupOf(popup, hands).onfocusout(event)}
          class="absolute top-full right-0 z-(--layer-popover) flex w-max min-w-[24ch] max-w-[36ch] flex-col rounded-card bg-raised p-tight shadow-float"
        >
          <Show
            when={naming()}
            fallback={
              <ul
                role="menu"
                tabindex="-1"
                aria-label={name()}
                class="max-h-palette overflow-y-auto"
                onKeyDown={walking(hands)}
              >
                <For each={items()} keyed={(item) => item.id}>
                  {(item) => (
                    <li role="none">
                      <button
                        type="button"
                        role="menuitem"
                        data-control=""
                        class="flex min-h-control-sm w-full items-center rounded-control px-snug py-tight text-left text-note text-pretty text-text hover:wash focus-visible:wash"
                        ref={(node) => itemNodes.set(item().id, node)}
                        onClick={() => item().act()}
                      >
                        {item().word}
                      </button>
                    </li>
                  )}
                </For>
              </ul>
            }
          >
            {(kind) => (
              <form class="flex flex-col gap-tight p-tight" onSubmit={submit}>
                <input
                  class="h-control-sm rounded-control bg-page px-snug text-note text-text"
                  ref={(node) => {
                    field = node;
                  }}
                  aria-label={say(
                    lang(),
                    kind() === "tag" ? "world_tag_name" : "world_rename_field",
                  )}
                  aria-invalid={wrong() ? "true" : "false"}
                  aria-describedby={hint}
                  maxlength={kind() === "tag" ? 24 : 80}
                  value={typed()}
                  onInput={(event) => setTyped(event.currentTarget.value)}
                  onKeyDown={(event) => {
                    if (event.key === "Escape" && !event.isComposing) {
                      event.preventDefault();
                      close();
                    }
                  }}
                />
                <p id={hint} class={["text-note", wrong() ? "text-alert" : "text-text-faint"]}>
                  {say(lang(), kind() === "tag" ? "world_tag_hint" : "world_rename_hint")}
                </p>
              </form>
            )}
          </Show>
        </div>
      </Show>
    </div>
  );
}
