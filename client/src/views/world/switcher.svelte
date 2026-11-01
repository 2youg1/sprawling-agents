<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // Zen's on-demand session tree. Current rooms navigate directly; past
  // sessions use the one branch-then-go door and the modal closes only
  // after the city accepts.
  import { say } from "../../core/lang";
  import { collapsedOf } from "../../core/session_tree";
  import { newestWorking } from "../../core/belief/live";
  import { cancel } from "../../core/commands";
  import { ui } from "../../ui";
  import { modalHold, modalWire } from "../modal";
  import type { Seat } from "../modal";
  import Modal from "../modal.look.svelte";
  import { branchFrom } from "../talk/branching";
  import { noteOf, waitWhy } from "../talk/branch_note";
  import { askStretches } from "./stretches.svelte";
  import { switchItems, switcherLookOf } from "./switcher";
  import type { SwitchItem } from "./switcher";
  import Look from "./switcher.look.svelte";

  interface Props {
    readonly onClose: () => void;
    readonly seat?: Seat | undefined;
  }
  const { onClose, seat = "modal" }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const stretches = askStretches(u);
  const branching = u.branches.held;
  // svelte-ignore state_referenced_locally (the modal's seat is fixed for its life)
  const hold = modalHold(seat);
  const sheet = $derived(
    modalWire({
      seat,
      named: { "aria-label": say($lang, "world_session_switcher") },
      backdrop: "closes",
      hold,
      onClose,
    }),
  );

  let query = $state("");
  let cursor = $state(0);
  let overrides = $state<Readonly<Record<string, boolean>>>({});
  const collapsed = $derived(collapsedOf(stretches.all, overrides));
  const rows = $derived(
    switchItems(stretches.all, query, collapsed, $lang, u.now()).map((row) => {
      if (row.origin === null) return row;
      const opening = branchFrom($branching, row.to.address, row.origin);
      const going = newestWorking($belief, row.to.address);
      return {
        ...row,
        note: noteOf(
          opening,
          row.to.address,
          $lang,
          going === undefined
            ? null
            : () => {
                u.send(cancel(going.run));
              },
        ),
        refused: waitWhy(opening),
      };
    }),
  );
  const uid = $props.id();
  const list = `${uid}-tree`;

  function open(row: SwitchItem): void {
    if (row.refused !== undefined) return;
    if (row.origin === null) {
      u.go(row.to);
      onClose();
    } else {
      u.branches.open(row.to.address, { kind: "branch", origin: row.origin }, onClose);
    }
  }

  const look = $derived(
    switcherLookOf(
      { query, cursor, collapsed, rows, list, lang: $lang },
      {
        write: (line) => {
          query = line;
          cursor = 0;
        },
        move: (at) => {
          cursor = at;
        },
        fold: (next) => {
          overrides = Object.fromEntries(
            rows.filter((row) => row.children).map((row) => [row.key, next.has(row.key)]),
          );
        },
        open,
        hover: (at) => {
          cursor = at;
        },
      },
    ),
  );
</script>

<Modal wire={sheet} {seat}>
  <Look {...look} />
</Modal>
