// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import type { Snippet } from "svelte";
import { createAttachmentKey } from "svelte/attachments";
import type { Attachment } from "svelte/attachments";

import type { Cancelling, SheetWire, Stands } from "./sheet";

export interface DialogProps {
  readonly open: boolean;
  
  readonly title: string;
  
  readonly detail?: string;
  readonly confirmLabel: string;
  readonly cancelLabel: string;
  readonly onConfirm: () => void;
  readonly onCancel: () => void;
  
  readonly destructive?: boolean;
  
  
  readonly children?: Snippet;
}

export interface Said {
  readonly id: string;
  readonly text: string;
}

export interface Answer {
  readonly label: string;
  readonly tone: "secondary" | "primary" | "destructive";
  readonly onPress: () => void;
}

export interface DialogLook {
  readonly sheet: SheetWire;
  readonly stands: Stands;
  readonly heading: Said;
  readonly detail: Said | undefined;
  readonly grounds: Snippet | undefined;
  
  
  readonly answers: readonly [Answer, Answer];
}


const HOLD = createAttachmentKey();

export function lookOf(
  props: DialogProps,
  uid: string,
  hold: Attachment<HTMLDialogElement>,
): DialogLook {
  const heading = { id: `${uid}-title`, text: props.title };
  const detail =
    props.detail === undefined ? undefined : { id: `${uid}-detail`, text: props.detail };
  return {
    sheet: {
      [HOLD]: hold,
      "aria-labelledby": heading.id,
      "aria-describedby": detail?.id,
      
      
      oncancel: (event: Cancelling) => {
        event.preventDefault();
        props.onCancel();
      },
    },
    stands: "centre",
    heading,
    detail,
    grounds: props.children,
    answers: [
      { label: props.cancelLabel, tone: "secondary", onPress: props.onCancel },
      {
        label: props.confirmLabel,
        tone: props.destructive === true ? "destructive" : "primary",
        onPress: props.onConfirm,
      },
    ],
  };
}
