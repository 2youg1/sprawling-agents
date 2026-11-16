<script lang="ts">
  import { keymap, marks } from "../../core/keys";
  import { say } from "../../core/lang";
  import { lineIn } from "../../core/stretches";
  import { ui } from "../../ui";
  import type { Address, Seq } from "../../wire";
  import { openPalette } from "../palette_door.svelte";
  import type { Arrival } from "../palette_door.svelte";
  import { addressLineOf, boxWords } from "./address_line";
  import type { At } from "./address_line";
  import Look from "./address_line.look.svelte";

  interface Props {
    readonly address: Address;

    readonly session?: Seq | undefined;

    readonly onOpen?: ((arrival: Arrival) => void) | undefined;

    readonly at?: At | undefined;
  }

  const { address, session, onOpen = openPalette, at = "head" }: Props = $props();

  const u = ui();
  const { lang } = u;
  const lines = $derived(u.conn.asking.ask({ sessions: { room: address } }));
  const found = u.prefs.paletteFound;
  const keys = keymap();
  const began = $derived(
    session ??
      lineIn($lines !== undefined && "sessions" in $lines ? $lines.sessions : undefined, undefined)
        ?.began ??
      null,
  );
</script>

<Look
  {...addressLineOf($lang, address, began, onOpen)}
  seat={at === "box" ? { at, ...boxWords($lang) } : { at }}
  lesson={$found
    ? undefined
    : {
        words: say($lang, "talk_address_hint"),
        chord: marks(keys.chord("palette"), keys.platform).join(" "),
      }}
/>
