<script lang="ts">
  import type { Key } from "../../core/lang";
  import { say } from "../../core/lang";
  import type { Keeper } from "../../core/prefs_city";
  import { ui } from "../../ui";
  import Badge from "../parts/badge.svelte";

  const WORDS: Record<Keeper, Key> = {
    browser: "setup_kept_browser",
    city: "setup_kept_city",
  };

  const { keeper }: { readonly keeper: Keeper } = $props();

  const { lang } = ui();
</script>

<Badge text={say($lang, WORDS[keeper])} />
