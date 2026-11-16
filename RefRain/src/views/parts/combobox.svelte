<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts">
  
  
  import { tick, untrack } from "svelte";
  import type { Attachment } from "svelte/attachments";

  import { lookOf, matching, rowId } from "./combobox";
  import type { Answer, Choice, ComboboxProps, Hands, Layer, Return } from "./combobox";
  import Look from "./combobox.look.svelte";
  import { revealIn, sideFor } from "./layer";
  import type { Side } from "./layer";

  
  const PREFERRED: Side = "below";

  const props: ComboboxProps = $props();
  const uid = $props.id();

  let layer = $state<Layer>(untrack(() => props.starts) === "open" ? "open" : "unopened");
  let query = $state("");
  
  
  let at = $state(0);
  
  let kept = $state<readonly Choice[]>([]);
  let side = $state<Side>(PREFERRED);

  let root = $state<HTMLDivElement | undefined>(undefined);
  let trigger = $state<HTMLButtonElement | undefined>(undefined);
  let panel = $state<HTMLDivElement | undefined>(undefined);
  let search = $state<HTMLInputElement | undefined>(undefined);
  let list = $state<HTMLUListElement | undefined>(undefined);

  const live = $derived(matching(props.choices, query));
  const rows = $derived(layer === "open" ? live : kept);
  const cursor = $derived(Math.max(0, Math.min(at, rows.length - 1)));

  
  function close(focus: Return): void {
    kept = live;
    layer = "shut";
    query = "";
    at = 0;
    if (focus === "opener") trigger?.focus();
  }

  function take(choice: Choice | undefined): void {
    if (choice === undefined) return;
    props.onPick(choice.value);
    close("opener");
  }

  function act(answered: Answer): void {
    switch (answered.kind) {
      case "cursor":
        at = answered.at;
        
        void tick().then(revealCursor);
        return;
      case "take":
        take(rows.at(cursor));
        return;
      case "close":
        close(answered.focus);
        return;
      case "pass":
        return;
    }
  }

  function revealCursor(): void {
    if (layer !== "open" || list === undefined || rows.length === 0) return;
    const row = document.getElementById(rowId(uid, cursor));
    if (row !== null) revealIn(list, row);
  }

  const keep =
    <E extends HTMLElement>(set: (node: E | undefined) => void): Attachment<E> =>
    (node) => {
      set(node);
      return () => {
        set(undefined);
      };
    };

  const hands: Hands = {
    toggle: () => {
      if (layer === "open") {
        close("opener");
        return;
      }
      side = PREFERRED;
      layer = "open";
      
      
      void tick().then(() => search?.focus({ preventScroll: true }));
    },
    act,
    type: (typed) => {
      
      query = typed;
      at = 0;
    },
    point: (index) => {
      at = index;
    },
    take,
    
    
    leave: (next) => {
      if (layer !== "open") return;
      if (next instanceof Node && root?.contains(next) === true) return;
      close("leave");
    },
    holds: {
      root: keep((node) => {
        root = node;
      }),
      trigger: keep((node) => {
        trigger = node;
      }),
      layer: keep((node) => {
        panel = node;
      }),
      search: keep((node) => {
        search = node;
      }),
      list: keep((node) => {
        list = node;
      }),
    },
  };

  const look = $derived(lookOf({ props, uid, layer, query, cursor, rows, side }, hands));

  
  $effect(() => {
    if (layer === "open" && panel !== undefined) side = sideFor(panel, PREFERRED);
  });

  
  $effect(revealCursor);
</script>

<Look {...look} />
