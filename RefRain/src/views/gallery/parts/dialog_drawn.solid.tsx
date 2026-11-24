// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { createUniqueId } from "solid-js";
import { say } from "../../../core/lang";
import { storeValue } from "../../../core/solid_store";
import { ui } from "../../../ui";
import { lookOf, type DialogProps } from "../../parts/dialog";
import DialogFrame from "../../parts/dialog.look.solid";
import { GalleryCase } from "../case";

export default function DrawnDialogs() {
  const lang = storeValue(ui().lang);
  const uid = createUniqueId();
  const quiet = (): void => undefined;
  const look = (destructive: boolean) => {
    const words: DialogProps<JSX.Element> = {
      open: true,
      title: say(lang(), "part_remove_endpoint"),
      ...(destructive ? { detail: say(lang(), "part_remove_detail") } : {}),
      confirmLabel: say(lang(), destructive ? "part_delete" : "part_save"),
      cancelLabel: say(lang(), "part_cancel"),
      destructive,
      onConfirm: quiet,
      onCancel: quiet,
    };
    return { ...lookOf(words, `${uid}-${String(destructive)}`), stands: "in-flow" as const };
  };
  return (
    <>
      <GalleryCase label="dialog · drawn open, destroying something, with what it costs">
        <DialogFrame look={look(true)} drawn />
      </GalleryCase>
      <GalleryCase label="dialog · drawn open, a plain confirmation with no detail">
        <DialogFrame look={look(false)} drawn />
      </GalleryCase>
    </>
  );
}
