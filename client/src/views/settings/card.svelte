<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // One settings card (client/Spec.lean §4-36), the seat: a title, one
  // line saying what it governs, its fields, and a foot with where the
  // save stands and, on a card saved by a press, the save button. A
  // card without `onSave` is saved by its own controls, one pick at a
  // time, and its foot says when a pick takes effect. A refusal keeps
  // the draft in the fields and says the city's own way on. The words
  // and the foot's reading are decided here (`card.ts`); the look draws
  // them.

  import type { Snippet } from "svelte";

  import { say } from "../../core/lang";
  import type { Key } from "../../core/lang";
  import { ui } from "../../ui";
  import { pressable, standingOf } from "./card";
  import Look from "./card.look.svelte";
  import type { Saving } from "./saving";

  interface Props {
    readonly title: Key;
    readonly note: Key;
    readonly saving: Saving;
    // When a saved change takes effect.
    readonly settled: Key;
    readonly onSave?: () => void;
    readonly children: Snippet;
  }

  const { title, note, saving, settled, onSave, children }: Props = $props();
  const { lang } = ui();
  const words = (key: Key): string => say($lang, key);

  const press = $derived(
    onSave === undefined
      ? null
      : {
          label: words("saving_save"),
          loading: saving.kind === "saving",
          why: pressable(saving) ? undefined : words("saving_nothing"),
          onPress: onSave,
        },
  );
</script>

<Look
  title={words(title)}
  note={words(note)}
  standing={standingOf(saving, words(settled), onSave !== undefined, words)}
  {press}
  {children}
/>
