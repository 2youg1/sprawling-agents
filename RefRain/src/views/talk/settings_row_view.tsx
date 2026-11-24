// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, For, onSettled, Show, untrack } from "solid-js";
import { say } from "../../core/lang";
import { buildingOf } from "../../core/route";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Address } from "../../wire";
import { focusCatalog, type CatalogFocus } from "../building/catalog_focus";
import type { PopoverBinding } from "../parts/popover";
import Popover from "../parts/popover_view";
import { HOLD } from "./fact";
import {
  childOf,
  offersOf,
  orderOf,
  parentOf,
  SECTION,
  type Combination,
  type Section,
  type Segment,
} from "./picker";
import { pickerOf, startOf, type PickerMenu, type PickerLook } from "./picker_look";
import { inForce, type PickerFacts, type PickerHeld } from "./picker_scene";
import { permissionsOf, type PermissionsLook } from "./policy";
import { RECENT_COMBINATIONS } from "./recent";
import Sandbox, { FACT } from "./sandbox_fact";
import { menuOf, sameBinding, type RowDraws, type RowMenu, type RowStarts } from "./settings_row";

const DOORS = [
  { key: "tools", letter: "A", word: "catalog_door_tools" },
  { key: "skills", letter: "B", word: "catalog_door_skills" },
  { key: "mcp", letter: "C", word: "catalog_door_mcp" },
  { key: "worktree", letter: "D", word: "catalog_door_worktree" },
] as const;

function Catalog(props: { readonly room: Address }) {
  const u = ui();
  const lang = storeValue(u.lang);
  function open(focus: CatalogFocus): void {
    const address = buildingOf(props.room);
    if (focus === "mcp") u.go({ kind: "mcp" });
    else if (focus === "worktree") u.go({ kind: "building", address });
    else {
      focusCatalog(focus);
      u.go({ kind: "building", address, section: "catalog" });
    }
  }
  return (
    <span
      class="flex min-w-0 flex-wrap items-center gap-hair px-tight text-note text-text-faint"
      role="group"
      aria-label={say(lang(), "bld_catalog")}
    >
      <For each={DOORS}>
        {(door) => (
          <>
            <Show when={door.key !== "tools"}>
              <span aria-hidden="true" class="text-text-faint">
                /
              </span>
            </Show>
            <button
              type="button"
              data-control="surface"
              class="inline-flex h-touch items-center gap-hair rounded-control px-tight"
              aria-label={`${door.letter} ${say(lang(), door.word)}`}
              onClick={() => open(door.key)}
            >
              <span class="font-mono text-text-quiet">{door.letter}</span>
              <span>{say(lang(), door.word)}</span>
            </button>
          </>
        )}
      </For>
    </span>
  );
}

export function ModelPicker(props: { readonly menu: PickerMenu }) {
  return (
    <Popover
      label="picker_title"
      columns={props.menu.columns}
      onApply={props.menu.onApply}
      onClose={props.menu.onClose}
      bind={props.menu.bind}
      onCursorChange={props.menu.onCursorChange}
      presentation={{
        sections: props.menu.sections,
        before: (column) =>
          column.id === SECTION.first ? (
            <input
              type={props.menu.filter.type}
              ref={(node) => props.menu.filter[HOLD]?.(node)}
              class="h-touch w-full min-w-0 shrink-0 rounded-control bg-page px-base text-body text-text outline-hidden placeholder:text-text-faint"
              aria-label={props.menu.filter["aria-label"]}
              placeholder={props.menu.filter.placeholder}
              aria-activedescendant={props.menu.filter["aria-activedescendant"]}
              aria-controls={props.menu.filter["aria-controls"]}
              value={props.menu.filter.value}
              onInput={(event) => props.menu.filter.oninput(event)}
              onKeyDown={(event) => props.menu.filter.onkeydown(event)}
            />
          ) : undefined,
        footer: () => (
          <Show when={props.menu.bandRoom}>
            <div class="invisible flex flex-col gap-tight" aria-hidden="true">
              <div class="h-control-sm" />
              <div class="h-touch" />
              <p class="line-clamp-2 min-h-[2lh]" />
            </div>
          </Show>
        ),
      }}
    />
  );
}

function Model(props: { readonly look: PickerLook }) {
  return (
    <div class="contents" ref={(node) => props.look.frame[HOLD]?.(node)}>
      <Show when={props.look.menu}>{(menu) => <ModelPicker menu={menu()} />}</Show>
      <button
        type="button"
        data-control=""
        ref={(node) => props.look.token[HOLD]?.(node)}
        aria-label={props.look.token["aria-label"]}
        aria-haspopup={props.look.token["aria-haspopup"]}
        aria-expanded={props.look.token["aria-expanded"] ? "true" : "false"}
        onClick={(event) => props.look.token.onclick(event)}
        class="token inline-flex h-touch min-w-0 max-w-full items-center rounded-control text-note text-text-quiet transition-[background-color,color,scale] ease-leave hover:bg-raised-hover hover:text-text hover:ease-arrive active:scale-(--scale-press) active:ease-arrive still:transition-none still:active:scale-100 aria-expanded:text-text"
      >
        <For each={props.look.segments} keyed={(part) => part.segment}>
          {(part, at) => (
            <>
              <Show when={at() > 0}>
                <span aria-hidden="true" class="shrink-0 text-text-faint">
                  ·
                </span>
              </Show>
              <span
                data-segment={part().segment}
                class={[
                  "inline-flex h-touch min-w-0 items-center rounded-control px-snug hover:wash",
                  part().open && "wash",
                  part().segment !== "model" && "shrink-0",
                ]}
              >
                <span class="truncate">{part().label}</span>
              </span>
            </>
          )}
        </For>
      </button>
    </div>
  );
}

function Permissions(props: { readonly look: PermissionsLook }) {
  return (
    <div
      class="flex min-w-0"
      ref={(node) => props.look.frame[HOLD]?.(node)}
      onFocusOut={(event) => props.look.frame.onfocusout(event)}
    >
      <button
        type="button"
        data-control=""
        ref={(node) => props.look.trigger[HOLD]?.(node)}
        aria-label={props.look.trigger["aria-label"]}
        aria-haspopup={props.look.trigger["aria-haspopup"]}
        aria-expanded={props.look.trigger["aria-expanded"] ? "true" : "false"}
        onClick={() => props.look.trigger.onclick()}
        class={[
          FACT,
          "hover:wash hover:text-text aria-expanded:wash aria-expanded:text-text transition-colors duration-short ease-leave hover:ease-arrive",
        ]}
      >
        <span class="truncate">{props.look.face}</span>
      </button>
      <Show when={props.look.panel}>
        {(panel) => (
          <div
            class="rise absolute bottom-full left-0 z-(--layer-popover) mb-snug flex w-full flex-col gap-base rounded-panel border border-edge-panel bg-raised p-base shadow-float"
            role={panel().wire.role}
            tabindex={panel().wire.tabindex}
            aria-label={panel().wire["aria-label"]}
            onKeyDown={(event) => panel().wire.onkeydown(event)}
          >
            <For each={panel().switches} keyed={(option) => option.key}>
              {(option) => (
                <label class="flex items-center justify-between gap-base text-body text-text">
                  {option().label}
                  <input
                    type={option().wire.type}
                    role={option().wire.role}
                    checked={option().wire.checked}
                    ref={(node) => option().wire[HOLD]?.(node)}
                    onChange={(event) => option().wire.onchange(event)}
                  />
                </label>
              )}
            </For>
            <div class="flex flex-wrap gap-base">
              <For each={panel().selects} keyed={(select) => select.key}>
                {(select) => (
                  <label class="flex min-w-0 flex-1 flex-col gap-tight text-note text-text-quiet">
                    {select().label}
                    <select
                      class="h-control w-full min-w-0 rounded-control bg-page px-snug text-body text-text"
                      value={select().wire.value}
                      onChange={(event) => select().wire.onchange(event)}
                    >
                      <For each={select().options} keyed={(option) => option.key}>
                        {(option) => <option value={option().key}>{option().label}</option>}
                      </For>
                    </select>
                  </label>
                )}
              </For>
            </div>
          </div>
        )}
      </Show>
    </div>
  );
}

export default function SettingsRow(props: {
  readonly picker: PickerFacts;
  readonly room: Address | null;
  readonly draws: RowDraws;
  readonly kept: boolean;
  readonly menu?: RowStarts;
}) {
  const u = ui();
  const lang = storeValue(u.lang);
  const policy = storeValue(u.policy);
  const choices = storeValue(RECENT_COMBINATIONS.choices);
  const starts = untrack(() => props.menu ?? "closed");
  const [menu, setMenu] = createSignal<RowMenu>(menuOf(starts));
  const [segment, setSegment] = createSignal<Segment | null>(
    starts === "open" || starts === "closed" ? null : starts,
  );
  const [pick, setPick] = createSignal<PickerHeld["pick"]>(null);
  const [parent, setParent] = createSignal<string | null>(null);
  const [query, setQuery] = createSignal("");
  const [whole, setWhole] = createSignal<readonly Section[]>([]);
  const [binding, setBinding] = createSignal<PopoverBinding | null>(null);
  const [active, setActive] = createSignal<string | null>(null);
  const [filter, setFilter] = createSignal<HTMLElement>();
  const [first, setFirst] = createSignal<HTMLElement>();
  const drawn: Record<"token" | "frame" | "permission" | "policyFrame", HTMLElement | undefined> = {
    token: undefined,
    frame: undefined,
    permission: undefined,
    policyFrame: undefined,
  };
  const point =
    (slot: keyof typeof drawn) =>
    (node: HTMLElement): void => {
      drawn[slot] = node;
    };
  function pinnedNow(): PickerHeld["pinned"] {
    const offers = offersOf(props.picker.endpoints);
    const order = orderOf(offers);
    const offer = offers.find(
      (each) =>
        each.endpoint === props.picker.chosen?.endpoint && each.model === props.picker.chosen.model,
    );
    return offer === undefined
      ? { first: undefined, second: undefined }
      : { first: parentOf(order, offer), second: childOf(order, offer) };
  }
  const [pinned, setPinned] = createSignal<PickerHeld["pinned"]>(untrack(pinnedNow));
  const [opened, setOpened] = createSignal<Combination | null>(
    untrack(() => inForce(props.picker)),
  );
  let preview = starts !== "closed";
  let drawnOpen = preview;
  let ended: Combination | null = null;
  createEffect(
    () => (menu() === "policy" ? first() : menu() === "model" ? filter() : undefined),
    (target) => {
      if (!target?.isConnected) return;
      if (preview) preview = false;
      else target.focus({ preventScroll: true });
    },
  );

  function close(focus: "token" | "stay"): void {
    setMenu(null);
    setBinding(null);
    setQuery("");
    if (ended !== null) RECENT_COMBINATIONS.keep(ended);
    ended = null;
    if (focus === "token") queueMicrotask(() => drawn.token?.focus());
  }
  const model = createMemo(() =>
    props.draws !== "everything"
      ? undefined
      : pickerOf(
          lang(),
          props.picker,
          {
            open: menu() === "model",
            segment: segment(),
            pick: pick(),
            parent: parent(),
            pinned: pinned(),
            opened: opened(),
            query: query(),
            whole: whole(),
            kept: choices(),
            binding: binding(),
            active: active(),
          },
          {
            open: (from) => {
              preview = false;
              drawnOpen = false;
              setFilter(undefined);
              setSegment(from);
              setPick(null);
              setParent(null);
              setQuery("");
              setWhole([]);
              setPinned(pinnedNow());
              setOpened(inForce(props.picker));
              ended = null;
              setMenu("model");
            },
            close,
            hold: (change) => {
              if (change.pick !== undefined) setPick(change.pick);
              if (change.parent !== undefined) setParent(change.parent);
              if (change.query !== undefined) setQuery(change.query);
              if (change.whole !== undefined) setWhole(change.whole);
            },
            keep: (combination) => {
              ended = combination;
            },
            bound: (next) => {
              const held = untrack(binding);
              if (sameBinding(held, next)) return;
              setBinding(next);
              if (held === null) next.pointColumn(startOf(props.picker, untrack(segment)));
            },
            cursor: (id) => {
              setActive(id);
            },
            focusFilter: () => {
              preview = false;
              filter()?.focus({ preventScroll: true });
            },
            holdToken: point("token"),
            holdFilter: (node) => {
              setFilter(node);
            },
            holdFrame: point("frame"),
          },
        ),
  );
  const permissions = createMemo(() =>
    props.draws !== "everything"
      ? undefined
      : permissionsOf(lang(), policy(), menu() === "policy", {
          choose: u.choosePolicy,
          toggle: () => {
            if (menu() === "policy") setMenu(null);
            else {
              setFirst(undefined);
              setMenu("policy");
            }
          },
          close: (focus) => {
            setMenu(null);
            if (focus === "opener") drawn.permission?.focus();
          },
          inside: (target) =>
            target instanceof Node && drawn.policyFrame?.contains(target) === true,
          hold: {
            frame: point("policyFrame"),
            trigger: point("permission"),
            first: (node) => {
              setFirst(node);
            },
          },
        }),
  );
  function outside(event: PointerEvent): void {
    if (
      menu() !== "model" ||
      drawnOpen ||
      (event.target instanceof Node && drawn.frame?.contains(event.target) === true)
    )
      return;
    model()?.menu?.onOutside();
  }
  onSettled(() => {
    window.addEventListener("pointerdown", outside);
    return () => window.removeEventListener("pointerdown", outside);
  });
  return (
    <Show when={props.draws === "everything" || props.kept}>
      <div class="relative flex flex-wrap items-center justify-between gap-tight pt-tight">
        <div class="edge-slot -ml-snug flex min-w-0 flex-wrap items-center narrow:ml-0">
          <Show when={props.room}>{(room) => <Catalog room={room()} />}</Show>
          <Show when={props.draws === "everything"}>
            <Sandbox room={props.room} />
          </Show>
          <Show when={props.kept}>
            <span class="px-snug text-note text-alert">{say(lang(), "talk_not_live")}</span>
          </Show>
        </div>
        <Show when={model() !== undefined || permissions() !== undefined}>
          <div class="-mr-snug ml-auto flex min-w-0 items-center narrow:mr-0">
            <Show when={model()}>{(look) => <Model look={look()} />}</Show>
            <Show when={permissions()}>{(look) => <Permissions look={look()} />}</Show>
          </div>
        </Show>
      </div>
    </Show>
  );
}
