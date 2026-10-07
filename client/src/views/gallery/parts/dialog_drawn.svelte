<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The irreversible question drawn open in its fold's flow, the way the
  // chord sheet's specimen is: a modal one would stand in the top layer
  // over every fold of this route, so the render gate never measured the
  // dialog's own box. Drawn from the same look the seat renders, with
  // the same wiring, only standing `in-flow` and `open`.
  import { say } from "../../../core/lang";
  import { ui } from "../../../ui";
  import DialogLook from "../../parts/dialog.look.svelte";
  import { lookOf, type DialogProps } from "../../parts/dialog";
  import Case from "../case.svelte";

  const { lang } = ui();
  const uid = $props.id();

  function drawn(props: DialogProps, at: string) {
    const look = lookOf(props, `${uid}-${at}`, () => undefined);
    return { ...look, stands: "in-flow" as const, sheet: { ...look.sheet, open: true } };
  }

  const NOTHING = (): void => undefined;
</script>

<Case label="dialog · drawn open, destroying something, with what it costs">
  <DialogLook
    {...drawn(
      {
        open: true,
        title: say($lang, "part_remove_endpoint"),
        detail: say($lang, "part_remove_detail"),
        confirmLabel: say($lang, "part_delete"),
        cancelLabel: say($lang, "part_cancel"),
        destructive: true,
        onConfirm: NOTHING,
        onCancel: NOTHING,
      },
      "destroying",
    )}
  />
</Case>

<Case label="dialog · drawn open, a plain confirmation with no detail">
  <DialogLook
    {...drawn(
      {
        open: true,
        title: say($lang, "part_remove_endpoint"),
        confirmLabel: say($lang, "part_save"),
        cancelLabel: say($lang, "part_cancel"),
        onConfirm: NOTHING,
        onCancel: NOTHING,
      },
      "plain",
    )}
  />
</Case>
