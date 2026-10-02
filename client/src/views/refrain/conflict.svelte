<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The draft stands on a version the city no longer holds. Nothing is
  // decided for the person (client/Spec.lean §4-46): they compare the draft
  // with the city's version, carry it over onto that version as a draft
  // still to be saved, or drop it - the last after a question, because
  // a dropped draft is gone (client D1). The bar carries the `asks` mark of
  // everything that waits on a person (7C) and is announced once.
  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import Button from "../parts/button.svelte";
  import Dialog from "../parts/dialog.svelte";
  import type { Session } from "./session.svelte";
  import { short } from "./reading";

  interface Props {
    readonly name: string;
    readonly session: Session;
    readonly onCompare: () => void;
  }

  const { name, session, onCompare }: Props = $props();

  const lang = ui().lang;

  let asking = $state(false);

  const theirs = $derived(session.theirs);
  // A version to compare with or move onto is needed first.
  const waiting = $derived(theirs === null ? { why: say($lang, "refrain_conflict_unknown") } : {});
</script>

<div class="refrain-conflict asks" role="alert">
  <p class="min-w-0 flex-1">
    {theirs === null
      ? say($lang, "refrain_conflict_unknown")
      : fill(say($lang, "refrain_conflict"), { version: short(theirs.version) })}
  </p>
  <Button label={say($lang, "refrain_compare")} tone="quiet" {...waiting} onPress={onCompare} />
  <Button
    label={say($lang, "refrain_rebase")}
    tone="secondary"
    {...waiting}
    onPress={() => {
      session.rebase();
    }}
  />
  <Button
    label={say($lang, "refrain_discard")}
    tone="destructive"
    {...waiting}
    onPress={() => {
      asking = true;
    }}
  />
</div>
<Dialog
  open={asking}
  title={fill(say($lang, "refrain_discard_title"), { name })}
  detail={say($lang, "refrain_discard_detail")}
  confirmLabel={say($lang, "refrain_discard")}
  cancelLabel={say($lang, "part_cancel")}
  destructive
  onConfirm={() => {
    asking = false;
    session.discard();
  }}
  onCancel={() => {
    asking = false;
  }}
/>
