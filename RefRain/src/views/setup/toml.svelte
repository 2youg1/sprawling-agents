<script lang="ts">
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
