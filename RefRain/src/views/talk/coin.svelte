<script lang="ts">
  import type { Sending } from "../../core/doing";
  import { ui } from "../../ui";
  import { lookOf } from "./coin";
  import type { Face } from "./coin_face";
  import Look from "./coin.look.svelte";

  interface Props {
    readonly face: Face;

    readonly sending: Sending;
    readonly onStop: () => void;
  }

  const { face, sending, onStop }: Props = $props();
  const { lang } = ui();
</script>

<Look {...lookOf(face, sending, $lang, onStop)} />
