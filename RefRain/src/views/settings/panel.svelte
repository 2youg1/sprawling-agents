

<script lang="ts">
  
  
  import type { SetupGroup, View } from "../../core/route";
  import { dialogOf } from "./panel";
  import Look from "./panel.look.svelte";
  import Sheet from "./sheet.svelte";

  interface Props {
    readonly group: SetupGroup;
    readonly beneath: View;
    readonly onPick: (group: SetupGroup) => void;
    readonly onClose: () => void;
    readonly onHome: () => void;
  }

  const { group, beneath, onPick, onClose, onHome }: Props = $props();

  const uid = $props.id();

  let sheet = $state<HTMLDialogElement | undefined>(undefined);
  const hold = (node: HTMLDialogElement): (() => void) => {
    sheet = node;
    return () => {
      sheet = undefined;
    };
  };

  
  $effect(() => {
    if (sheet === undefined || sheet.open) return;
    sheet.showModal();
    sheet.querySelector<HTMLElement>("nav [aria-current=true]")?.focus();
  });
</script>

<Look dialog={dialogOf(`${uid}-title`, onClose, hold)}>
  <Sheet {group} {beneath} {onPick} {onHome} titleId={`${uid}-title`} />
</Look>
