<script lang="ts">
  import { ui } from "../../ui";
  import { lookOf } from "./reasoning";
  import Look from "./reasoning.look.svelte";

  interface Props {
    readonly text: string;
    readonly open: boolean;
  }

  const { text, open }: Props = $props();
  const { lang } = ui();

  let shown = $state<string | null>(null);
  $effect(() => {
    const next = text;
    const id = requestAnimationFrame(() => {
      shown = next;
    });
    return () => {
      cancelAnimationFrame(id);
    };
  });

  const look = $derived(lookOf($lang, shown ?? text, open));
</script>

<Look {...look} />
