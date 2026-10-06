<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // One privacy entry: the seat. It turns the host's reading of one
  // control and where this page's operation on it stands into the
  // entry's look, and hands the look one callback per action. Which
  // look draws it is a prop, so a look from a UI library that takes
  // `EntryLook` replaces this one without touching the wiring; the
  // gallery draws the same entry through a second look to hold that
  // (`gallery/privacy.svelte`).

  import type { Component } from "svelte";

  import { ui } from "../../../ui";
  import type { Action, EntryLook } from "./entry";
  import ShippedLook from "./entry.look.svelte";
  import { lookOf, type EntryModel } from "./page";
  import type { Held } from "./state.svelte";

  interface Props {
    readonly model: EntryModel;
    readonly held: Held;
    readonly onPress: (action: Action) => void;
    readonly look?: Component<EntryLook>;
    // The id root the entry's id hangs off; the page passes its own so its
    // links can find the entry.
    readonly root?: string;
  }

  const uid = $props.id();
  const { model, held, onPress, look = ShippedLook, root = uid }: Props = $props();
  const { lang } = ui();

  const drawn = $derived(lookOf($lang, model, { root, stage: held.stage, shown: held.shown, press: onPress }));
  const Look = $derived(look);
</script>

<Look {...drawn} />
