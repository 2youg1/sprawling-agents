<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- The settings row under the composer's line (docs/frontend-method.md
§7I, client/Spec.lean §4-60): before a session starts, the workspace
chip with the sandbox beside it on the left, the model picker's token
and the permissions entry on the right; once one has, nothing but the
notice that a sentence the link did not take was kept in the box.
This file is the seat (client D95): it holds which menu is open, what
the open picker has chosen, typed and opened in full, and the drawn
elements, and moves the focus. `picker_look.ts` and `policy.ts` build
the value `settings_row.look.svelte` draws. It writes no class. -->
<script lang="ts">
  import { untrack } from "svelte";
  import type { Attachment } from "svelte/attachments";

  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";
  import type { PopoverBinding } from "../parts/popover";
  import type { Pill } from "./composer";
  import Listening from "./listening.svelte";
  import PillView from "./pill.svelte";
  import { childOf, offersOf, orderOf, parentOf } from "./picker";
  import type { Section, Segment } from "./picker";
  import { pickerOf, startOf } from "./picker_look";
  import type { PickerFacts, PickerHeld } from "./picker_look";
  import { permissionsOf } from "./policy";
  import { RECENT_COMBINATIONS } from "./recent.svelte";
  import Sandbox from "./sandbox.svelte";
  import { menuOf, sameBinding } from "./settings_row";
  import type { RowDraws, RowMenu, RowStarts, SettingsRowLook } from "./settings_row";
  import Look from "./settings_row.look.svelte";

  interface Props {
    readonly workspace: Pill;
    readonly picker: PickerFacts;
    readonly room: Address | null;
    readonly draws: RowDraws;
    readonly kept: boolean;
    // The menu a fixture draws open; it takes no focus until somebody
    // opens a menu themselves.
    readonly menu?: RowStarts;
  }

  const { workspace, picker, room, draws, kept, menu: starts = "closed" }: Props = $props();
  const u = ui();
  const { lang } = u;
  const policy = u.policy;

  const startSegment = (from: RowStarts): Segment | null => {
    switch (from) {
      case "model":
      case "provider":
      case "level":
        return from;
      case "open":
      case "closed":
        return null;
    }
  };

  let menu = $state<RowMenu>(menuOf(untrack(() => starts)));
  let preview = untrack(() => starts) !== "closed";
  // A picker a fixture drew open stays open under presses elsewhere on
  // the gallery page, until somebody opens one themselves.
  let drawnOpen = untrack(() => starts) !== "closed";
  let segment = $state<Segment | null>(startSegment(untrack(() => starts)));
  let pick = $state<PickerHeld["pick"]>(null);
  let parent = $state<string | null>(null);
  let query = $state("");
  let whole = $state<readonly Section[]>([]);
  let binding = $state<PopoverBinding | null>(null);
  let active = $state<string | null>(null);

  // The entries in force when the picker opens, kept inside the first
  // five rows of their lists for as long as it stays open.
  const pinnedNow = (): PickerHeld["pinned"] => {
    const offers = offersOf(picker.endpoints);
    const order = orderOf(offers);
    const offer = offers.find((each) => each.endpoint === picker.chosen?.endpoint && each.model === picker.chosen.model);
    return offer === undefined ? { first: undefined, second: undefined } : { first: parentOf(order, offer), second: childOf(order, offer) };
  };
  let pinned = $state<PickerHeld["pinned"]>(untrack(pinnedNow));

  // The elements no draw reads: a plain record, so the attachments that
  // fill it never write during a derivation.
  const drawn: Record<"token" | "frame" | "permission" | "policyFrame", HTMLElement | undefined> = {
    token: undefined,
    frame: undefined,
    permission: undefined,
    policyFrame: undefined,
  };
  const keep = (slot: keyof typeof drawn): Attachment<HTMLElement> => (node) => {
    drawn[slot] = node;
    return () => {
      if (drawn[slot] === node) drawn[slot] = undefined;
    };
  };
  const hold = { token: keep("token"), frame: keep("frame"), permission: keep("permission"), policyFrame: keep("policyFrame") };
  // The two elements an opened panel gives the focus to, held as state,
  // because the panel's content is drawn after it opens and the focus
  // waits for it.
  let filter = $state<HTMLElement | undefined>(undefined);
  let first = $state<HTMLElement | undefined>(undefined);
  const holdFilter: Attachment<HTMLElement> = (node) => {
    filter = node;
    return () => {
      if (filter === node) filter = undefined;
    };
  };
  const holdFirst: Attachment<HTMLElement> = (node) => {
    first = node;
    return () => {
      if (first === node) first = undefined;
    };
  };

  // An opened panel takes the focus: the permissions panel on its first
  // switch, the picker in its filter box. A panel a fixture drew open
  // does not, once.
  $effect(() => {
    if (menu !== "policy" || first === undefined) return;
    if (preview) {
      preview = false;
      return;
    }
    first.focus();
  });
  $effect(() => {
    if (menu !== "model" || filter === undefined) return;
    if (preview) {
      preview = false;
      return;
    }
    filter.focus({ preventScroll: true });
  });

  const closePicker = (focus: "token" | "stay"): void => {
    menu = null;
    binding = null;
    query = "";
    if (focus === "token") queueMicrotask(() => drawn.token?.focus());
  };

  const look: SettingsRowLook = $derived({
    facts: draws === "everything" ? facts : undefined,
    notLive: kept ? say($lang, "talk_not_live") : undefined,
    model:
      draws === "everything"
        ? pickerOf(
            $lang,
            picker,
            { open: menu === "model", segment, pick, parent, pinned, query, whole, kept: RECENT_COMBINATIONS.kept, binding, active },
            {
              open: (from) => {
                preview = false;
                drawnOpen = false;
                segment = from;
                pick = null;
                parent = null;
                query = "";
                whole = [];
                pinned = pinnedNow();
                menu = "model";
              },
              close: closePicker,
              hold: (change) => {
                if (change.pick !== undefined) pick = change.pick;
                if (change.parent !== undefined) parent = change.parent;
                if (change.query !== undefined) query = change.query;
                if (change.whole !== undefined) whole = change.whole;
              },
              keep: (combination) => {
                RECENT_COMBINATIONS.keep(combination);
              },
              bound: (next) => {
                if (untrack(() => sameBinding(binding, next))) return;
                const initial = untrack(() => binding === null);
                binding = next;
                if (initial) next.pointColumn(startOf(picker, untrack(() => segment)));
              },
              cursor: (id) => {
                active = id;
              },
              focusFilter: () => {
                preview = false;
                filter?.focus({ preventScroll: true });
              },
              holdToken: hold.token,
              holdFilter,
              holdFrame: hold.frame,
            },
          )
        : undefined,
    permissions:
      draws === "everything"
        ? permissionsOf($lang, $policy, menu === "policy", {
            choose: u.choosePolicy,
            toggle: () => {
              menu = menu === "policy" ? null : "policy";
            },
            close: (focus) => {
              menu = null;
              if (focus === "opener") drawn.permission?.focus();
            },
            inside: (target) => target instanceof Node && drawn.policyFrame?.contains(target) === true,
            hold: { frame: hold.policyFrame, trigger: hold.permission, first: holdFirst },
          })
        : undefined,
  });

  // A press anywhere outside the token and the open picker closes it,
  // and the press goes on to whatever it landed on.
  const outside = (event: PointerEvent): void => {
    if (menu !== "model" || drawnOpen) return;
    if (event.target instanceof Node && drawn.frame?.contains(event.target) === true) return;
    look.model?.menu?.onOutside();
  };
</script>

<svelte:window onpointerdown={outside} />

{#snippet listening()}
  {#if room !== null}<Listening {room} />{/if}
{/snippet}

{#snippet facts()}
  {#if workspace.choices.length > 0}
    <PillView spec={workspace} told={room === null ? undefined : listening} />
  {/if}
  <Sandbox {room} />
{/snippet}

{#if draws === "everything" || kept}
  <Look {...look} />
{/if}
