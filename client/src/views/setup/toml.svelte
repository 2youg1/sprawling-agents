<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The city's own `CONFIG.toml`, as the city's tree holds it.
  //
  // **A proofing tool, not a setting.** It is collapsed at the foot of
  // every group rather than standing as a column: a block that costs the
  // cards their width is a block nobody reads (client/Spec.lean §4-36). What
  // stands inside is the file itself, read through `Query::Document`
  // and never spelled by this page, so it cannot disagree with the
  // file; a setting's value and the layer it came from are answered
  // beside the control that owns it (4-30).

  import { readDocument } from "../../core/document";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import EmptyState from "../parts/empty.svelte";
  import { CITY_CONFIG } from "../settings/files";
  import TomlLook from "./toml.look.svelte";

  const u = ui();
  const { lang } = u;
  const file = u.conn.asking.ask({ document: { at: CITY_CONFIG } });
  const read = $derived(readDocument($file));
</script>

{#snippet unread()}
  <EmptyState missing="setup_toml_unread" />
{/snippet}

<TomlLook
  region={{ "aria-label": say($lang, "setup_toml") }}
  toggle={say($lang, "setup_toml_toggle")}
  path={CITY_CONFIG}
  text={read.kind === "held" && read.value.text !== "" ? read.value.text : undefined}
  {unread}
/>
