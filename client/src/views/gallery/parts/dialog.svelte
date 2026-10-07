<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The modal that confirms what cannot be undone, opened by the button
  // that asks for it.
  import { say } from "../../../core/lang";
  import { ui } from "../../../ui";
  import Button from "../../parts/button.svelte";
  import Dialog from "../../parts/dialog.svelte";
  import Case from "../case.svelte";

  const { lang } = ui();

  let asking = $state(false);
</script>

<Case label="dialog · confirming what cannot be undone">
  <Button
    label={say($lang, "setup_remove")}
    tone="destructive"
    onPress={() => {
      asking = true;
    }}
  />
  <Dialog
    open={asking}
    title={say($lang, "part_remove_endpoint")}
    detail={say($lang, "part_remove_detail")}
    confirmLabel={say($lang, "part_delete")}
    cancelLabel={say($lang, "part_cancel")}
    destructive
    onConfirm={() => {
      asking = false;
    }}
    onCancel={() => {
      asking = false;
    }}
  />
</Case>
