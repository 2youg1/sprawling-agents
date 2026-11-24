// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createSignal } from "solid-js";
import { say } from "../../../core/lang";
import { ui } from "../../../ui";
import Button from "../../parts/button.solid";
import Dialog from "../../parts/dialog_view";
import { GalleryCase } from "../case";
import { storeValue } from "../../../core/solid_store";

export default function DialogCases() {
  const lang = storeValue(ui().lang);
  const [asking, setAsking] = createSignal(false);
  return (
    <>
      <GalleryCase label="dialog · confirming what cannot be undone">
        <Button
          label={say(lang(), "setup_remove")}
          tone="destructive"
          onPress={() => {
            setAsking(true);
          }}
        />
        <Dialog
          open={asking()}
          title={say(lang(), "part_remove_endpoint")}
          detail={say(lang(), "part_remove_detail")}
          confirmLabel={say(lang(), "part_delete")}
          cancelLabel={say(lang(), "part_cancel")}
          destructive
          onConfirm={() => {
            setAsking(false);
          }}
          onCancel={() => {
            setAsking(false);
          }}
        />
      </GalleryCase>
    </>
  );
}
