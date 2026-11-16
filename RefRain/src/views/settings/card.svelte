

<script lang="ts">
  
  
  import type { Snippet } from "svelte";

  import { say } from "../../core/lang";
  import type { Key } from "../../core/lang";
  import { ui } from "../../ui";
  import { pressable, standingOf } from "./card";
  import Look from "./card.look.svelte";
  import type { Saving } from "./saving";

  interface Props {
    readonly title: Key;
    readonly note: Key;
    readonly saving: Saving;
    
    readonly settled: Key;
    readonly onSave?: () => void;
    readonly children: Snippet;
  }

  const { title, note, saving, settled, onSave, children }: Props = $props();
  const { lang } = ui();
  const words = (key: Key): string => say($lang, key);

  const press = $derived(
    onSave === undefined
      ? null
      : {
          label: words("saving_save"),
          loading: saving.kind === "saving",
          why: pressable(saving) ? undefined : words("saving_nothing"),
          onPress: onSave,
        },
  );
</script>

<Look
  title={words(title)}
  note={words(note)}
  standing={standingOf(saving, words(settled), onSave !== undefined, words)}
  {press}
  {children}
/>
