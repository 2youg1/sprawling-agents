<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
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
    readonly root?: string;
  }

  const uid = $props.id();
  const { model, held, onPress, look = ShippedLook, root = uid }: Props = $props();
  const { lang } = ui();

  const drawn = $derived(
    lookOf($lang, model, { root, stage: held.stage, shown: held.shown, press: onPress }),
  );
  const Look = $derived(look);
</script>

<Look {...drawn} />
