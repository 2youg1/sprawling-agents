<script lang="ts" module>
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Skeleton from "../parts/skeleton.svelte";

  const { lang } = ui();
</script>

<Skeleton label={say($lang, "machine_checking")} rows={12} tall />
