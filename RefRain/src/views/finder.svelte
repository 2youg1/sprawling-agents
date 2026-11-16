<script lang="ts">
  import type { Address } from "../wire";
  import Search from "./finder/search.svelte";
  import { modalHold, modalWire } from "./modal";
  import Modal from "./modal.look.svelte";

  interface Props {
    readonly under: Address;
    readonly onClose: () => void;
  }

  const { under, onClose }: Props = $props();

  const uid = $props.id();
  const wire = modalWire({
    seat: "modal",
    named: { "aria-labelledby": `${uid}-title` },
    backdrop: "stays",
    hold: modalHold("modal"),
    onClose: () => {
      onClose();
    },
  });
</script>

<Modal {wire} seat="modal">
  <Search {under} titleId="{uid}-title" {onClose} />
</Modal>
