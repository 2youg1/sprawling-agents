<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
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

    readonly path: string;
    readonly point: GitOid;
  }

  const { building, path, point }: Props = $props();

  const u = ui();
  const lang = u.lang;

  let asking = $state(false);

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
