<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- The settings row under the composer's line (docs/frontend-method.md
§7I, client/Spec.lean §4-60): before a session starts, the workspace
chip with the sandbox beside it on the left, the model entry and the
permissions entry on the right; once one has, nothing but the notice
that a sentence the link did not take was kept in the box.
This file is the seat (client D95): it holds which menu is open, the
provider pointed at, the typed filter and the drawn elements, and moves
the focus. `settings_row.ts` and `policy.ts` build the value
`settings_row.look.svelte` draws. It writes no class. -->
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
  import { permissionsOf } from "./policy";
  import Sandbox from "./sandbox.svelte";
  import { COLUMN, menuOf, modelOf, parentOf } from "./settings_row";
  import type { RowDraws, RowMenu, RowStarts, SettingsRowLook } from "./settings_row";
  import Look from "./settings_row.look.svelte";

  interface Props {
    readonly specs: readonly [Pill, Pill, Pill];
    readonly room: Address | null;
    readonly draws: RowDraws;
    readonly kept: boolean;
    // The menu a fixture draws open; it takes no focus until somebody
    // opens a menu themselves.
    readonly menu?: RowStarts;
  }

  const { specs, room, draws, kept, menu: starts = "closed" }: Props = $props();
  const u = ui();
  const { lang } = u;
  const policy = u.policy;

  let menu = $state<RowMenu>(menuOf(untrack(() => starts)));
  let preview = untrack(() => starts) !== "closed";
  let pointed = $state<string | null>(null);
  let query = $state("");
  let binding = $state<PopoverBinding | null>(null);
  let active = $state<string | null>(null);

  // The two entries' triggers and the frame of the permissions entry,
  // which no draw reads: a plain record, so the attachments that fill it
  // never write during a derivation.
  const drawn: Record<"model" | "permission" | "frame", HTMLElement | undefined> = { model: undefined, permission: undefined, frame: undefined };
  const keep = (slot: "model" | "permission" | "frame"): Attachment<HTMLElement> => (node) => {
    drawn[slot] = node;
    return () => {
      if (drawn[slot] === node) drawn[slot] = undefined;
    };
  };
  const hold = { model: keep("model"), permission: keep("permission"), frame: keep("frame") };
  // The two elements an opened panel gives the focus to, held as state,
  // because the panel's content is drawn after it opens and the focus
  // waits for it.
  let search = $state<HTMLElement | undefined>(undefined);
  let first = $state<HTMLElement | undefined>(undefined);
  const holdSearch: Attachment<HTMLElement> = (node) => {
    search = node;
    return () => {
      if (search === node) search = undefined;
    };
  };
  const holdFirst: Attachment<HTMLElement> = (node) => {
    first = node;
    return () => {
      if (first === node) first = undefined;
    };
  };

  // An opened panel takes the focus: the permissions panel on its first
  // switch, the model panel in its search box, again whenever another
  // provider's row brings one. A panel a fixture drew open does not,
  // once.
  $effect(() => {
    if (menu !== "policy" || first === undefined) return;
    if (preview) {
      preview = false;
      return;
    }
    first.focus();
  });
  $effect(() => {
    if (menu !== "model" || search === undefined || parentOf(pointed, specs[0]) === undefined) return;
    if (preview) {
      preview = false;
      return;
    }
    search.focus({ preventScroll: true });
  });

  const look: SettingsRowLook = $derived({
    facts: draws === "everything" ? facts : undefined,
    notLive: kept ? say($lang, "talk_not_live") : undefined,
    model:
      draws === "everything"
        ? modelOf(
            { lang: $lang, models: specs[0], effort: specs[2] },
            { open: menu === "model", pointed, query, binding, active },
            {
              toggle: () => {
                preview = false;
                query = "";
                menu = menu === "model" ? null : "model";
              },
              close: () => {
                menu = null;
                query = "";
                binding = null;
                queueMicrotask(() => {
                  drawn.model?.focus();
                });
              },
              point: (provider) => {
                pointed = provider;
                query = "";
              },
              query: (text) => {
                query = text;
              },
              bound: (next) => {
                const initial = untrack(() => binding?.controls.at(0) !== next.controls.at(0));
                binding = next;
                if (initial) next.pointColumn(COLUMN.model);
              },
              cursor: (id) => {
                active = id;
              },
              focus: {
                trigger: () => {
                  preview = false;
                  drawn.model?.focus();
                },
                search: () => {
                  preview = false;
                  search?.focus({ preventScroll: true });
                },
              },
              hold: { trigger: hold.model, search: holdSearch },
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
            inside: (target) => target instanceof Node && drawn.frame?.contains(target) === true,
            hold: { frame: hold.frame, trigger: hold.permission, first: holdFirst },
          })
        : undefined,
  });
</script>

{#snippet listening()}
  {#if room !== null}<Listening {room} />{/if}
{/snippet}

{#snippet facts()}
  {#if specs[1].choices.length > 0}
    <PillView spec={specs[1]} told={room === null ? undefined : listening} />
  {/if}
  <Sandbox {room} />
{/snippet}

{#if draws === "everything" || kept}
  <Look {...look} />
{/if}
