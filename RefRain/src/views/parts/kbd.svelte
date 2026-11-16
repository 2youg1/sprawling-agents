<script module>
  import type { Action } from "../../core/keys";
  import { keymap, marks } from "../../core/keys";
  import type { LineMove } from "../../core/lines";
  import { lineFaces } from "../../core/lines";
  import KbdLook from "./kbd.look.svelte";

  export type KbdProps = (
    { readonly action: Action } | { readonly move: LineMove } | { readonly initial: string }
  ) & { readonly class?: string };

  export { Kbd };

  function chordMarks(props: KbdProps): readonly string[] {
    const keys = keymap();
    if ("move" in props) return lineFaces(props.move).slice(0, 1);
    if ("initial" in props)
      return marks({ accel: false, shift: false, key: props.initial }, keys.platform);
    return marks(keys.chord(props.action), keys.platform);
  }
</script>

<script lang="ts">
  import type { Attachment } from "svelte/attachments";

  import { ACTIONS, LABELS } from "../../core/keys";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Look from "./kbd_sheet.look.svelte";
  import { sheetOf, type Seat } from "./kbd";

  interface CheatsheetProps {
    readonly seat: Seat;
    readonly onClose: () => void;
  }

  const uid = $props.id();
  const { lang } = ui();
  const { seat, onClose }: CheatsheetProps = $props();

  let sheet = $state<HTMLDialogElement | undefined>(undefined);
  const hold: Attachment<HTMLDialogElement> = (node) => {
    sheet = node;
    return () => {
      sheet = undefined;
    };
  };

  $effect(() => {
    const node = sheet;
    if (node === undefined || seat === "specimen") {
      return;
    }
    node.showModal();
    return () => {
      node.close();
    };
  });

  const look = $derived(
    sheetOf(
      { uid, seat, onClose, hold },
      {
        title: say($lang, "keys_title"),
        dismiss: say($lang, "dismiss"),
        where: say($lang, "keys_where"),
        rows: ACTIONS.map((action) => ({
          key: action,
          label: say($lang, LABELS[action]),
          marks: chordMarks({ action }),
        })).filter((row) => row.marks.length > 0),
      },
    ),
  );
</script>

{#snippet Kbd(props: KbdProps)}
  <KbdLook marks={chordMarks(props)} class={props.class} />
{/snippet}

<Look {...look} />
