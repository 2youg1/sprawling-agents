<script lang="ts">
  import { say } from "../../core/lang";
  import type { Pane } from "../../core/workbench";
  import { ui } from "../../ui";
  import { leaveSheet } from "../sheets.svelte";
  import { drawnElements } from "./drawn";
  import { sheetHeadOf } from "./sheet_head";
  import type { SheetHeadLook } from "./sheet_head";
  import Look from "./sheet_head.look.svelte";

  interface Props {
    readonly tabs: readonly { readonly pane: Pane; readonly label: string }[];
    readonly shown: Pane;

    readonly prefix: string;
    readonly onShow: (pane: Pane) => void;
  }

  const { tabs, shown, prefix, onShow }: Props = $props();
  const { lang } = ui();

  const drawn = drawnElements();

  const look: SheetHeadLook = $derived(
    sheetHeadOf(
      {
        tabs,
        shown,
        prefix,
        words: { back: say($lang, "world_back"), strip: say($lang, "world_panes") },
      },
      {
        leave: () => {
          leaveSheet("world");
        },
        show: onShow,
        focus: (pane) => drawn.get(pane)?.focus(),
        hold: drawn.hold,
      },
    ),
  );
</script>

<Look {...look} />
