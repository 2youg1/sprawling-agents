// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Option, Schema } from "effect";
import { createSignal } from "solid-js";
import { restoreFile } from "../../core/commands";
import { fill, say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import { Address, type GitOid } from "../../wire";
import { shortOid } from "../changes";
import Button from "../parts/button.solid";
import Dialog from "../parts/dialog_view";

interface Props {
  readonly building: Address;
  readonly path: string;
  readonly point: GitOid;
}

export default function TakeBack(props: Props) {
  const u = ui();
  const lang = storeValue(u.lang);
  const [asking, setAsking] = createSignal(false);
  const at = () =>
    Option.getOrNull(Schema.decodeOption(Address)(`${props.building}/${props.path}`));
  const slots = () => ({ path: props.path, oid: shortOid(props.point) });

  return (
    <>
      <Button
        label={say(lang(), "take_back")}
        tone="quiet"
        {...(at() === null ? { why: say(lang(), "take_back_not_in_tree") } : {})}
        onPress={() => {
          setAsking(true);
        }}
      />
      <Dialog
        open={asking()}
        title={fill(say(lang(), "take_back_title"), slots())}
        detail={fill(say(lang(), "take_back_detail"), slots())}
        confirmLabel={say(lang(), "take_back")}
        cancelLabel={say(lang(), "part_cancel")}
        destructive
        onConfirm={() => {
          setAsking(false);
          const target = at();
          if (target !== null) u.send(restoreFile(target, props.point));
        }}
        onCancel={() => {
          setAsking(false);
        }}
      />
    </>
  );
}
