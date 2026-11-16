<script lang="ts">
  import Laid from "../editor/laid.svelte";
  import { laidReply } from "../reply.svelte";
  import Look from "./saying.look.svelte";
  import NotePlace from "./note_place.svelte";
  import { faded } from "./saying";

  interface Props {
    readonly text: string;
    readonly who: string;
  }

  const { text, who }: Props = $props();

  const laid = laidReply(() => text, "streaming");
</script>

{#snippet blocks()}
  <Laid blocks={laid.blocks} />
{/snippet}

<NotePlace rhythm="shape">
  <Look {who} laid={blocks} {...faded(text.slice(laid.reached))} />
</NotePlace>
