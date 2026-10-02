<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // Take one file of this building back to what one checkpoint holds
  // (client-SPEC 4-50, UC8). The command rewrites the file in the
  // working tree, and what the file says now is in no checkpoint, so it
  // is asked through `parts/dialog` before it leaves (12-1). The city
  // refuses while a run works in the building; that refusal is a toast,
  // because nothing on this row can answer it.
  import { Option, Schema } from "effect";

  import { restoreFile } from "../../core/commands";
  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import { Address, type GitOid } from "../../wire";
  import { shortOid } from "../changes";
  import Button from "../parts/button.svelte";
  import Dialog from "../parts/dialog.svelte";

  interface Props {
    readonly building: Address;
    // The file's path inside the building.
    readonly path: string;
    readonly point: GitOid;
  }

  const { building, path, point }: Props = $props();

  const u = ui();
  const lang = u.lang;

  let asking = $state(false);

  // A path the address grammar does not take is not a file of the
  // city's tree, so the control says why it cannot rather than sending
  // a command the city would refuse.
  const at = $derived(Option.getOrNull(Schema.decodeOption(Address)(`${building}/${path}`)));
  const slots = $derived({ path, oid: shortOid(point) });
</script>

<Button
  label={say($lang, "take_back")}
  tone="quiet"
  {...at === null ? { why: say($lang, "take_back_not_in_tree") } : {}}
  onPress={() => {
    asking = true;
  }}
/>
<Dialog
  open={asking}
  title={fill(say($lang, "take_back_title"), slots)}
  detail={fill(say($lang, "take_back_detail"), slots)}
  confirmLabel={say($lang, "take_back")}
  cancelLabel={say($lang, "part_cancel")}
  destructive
  onConfirm={() => {
    asking = false;
    if (at !== null) u.send(restoreFile(at, point));
  }}
  onCancel={() => {
    asking = false;
  }}
/>
