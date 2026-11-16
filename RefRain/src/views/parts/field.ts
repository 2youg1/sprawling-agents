// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


export interface FieldProps {
  
  readonly label: string;
  
  
  readonly labelling?: "above" | "hidden";
  readonly value: string;
  readonly onInput: (value: string) => void;
  readonly help?: string;
  
  readonly error?: string;
  
  readonly prefix?: string;
  readonly suffix?: string;
  readonly placeholder?: string;
  
  
  readonly mono?: boolean;
  
  
  readonly kind?: "text" | "password" | "number";
  
  readonly step?: number;
  
  readonly pattern?: string;
  
  
  readonly describedBy?: string;
  
  
  readonly disabled?: boolean;
}


export interface Typed {
  readonly currentTarget: { value: string };
}


export interface LabelWire {
  readonly for: string;
}


export interface BoxWire {
  readonly id: string;
  readonly type: "text" | "password" | "number";
  readonly inputmode: "numeric" | undefined;
  readonly step: number | undefined;
  readonly pattern: string | undefined;
  readonly value: string;
  readonly placeholder: string | undefined;
  readonly "aria-disabled": boolean;
  readonly "aria-invalid": boolean;
  readonly "aria-describedby": string | undefined;
  readonly oninput: (event: Typed) => void;
}


export interface NoteLook {
  readonly kind: "error" | "help";
  readonly text: string;
  
  readonly wire: { readonly id: string; readonly role: "alert" | undefined };
}

export interface FieldLook {
  readonly label: string;
  readonly labelling: "above" | "hidden";
  readonly prefix: string | undefined;
  readonly suffix: string | undefined;
  readonly mono: boolean;
  readonly disabled: boolean;
  
  readonly refused: boolean;
  readonly labelWire: LabelWire;
  readonly box: BoxWire;
  readonly note: NoteLook | undefined;
}


export function describedOf(props: FieldProps, note: string): string | undefined {
  const own = props.help === undefined && props.error === undefined ? [] : [note];
  const ids = [...(props.describedBy === undefined ? [] : [props.describedBy]), ...own];
  return ids.length === 0 ? undefined : ids.join(" ");
}

function noteOf(props: FieldProps, id: string): NoteLook | undefined {
  if (props.error !== undefined)
    return { kind: "error", text: props.error, wire: { id, role: "alert" } };
  if (props.help !== undefined)
    return { kind: "help", text: props.help, wire: { id, role: undefined } };
  return undefined;
}


export function lookOf(props: FieldProps, uid: string): FieldLook {
  const box = `${uid}-box`;
  const note = `${uid}-note`;
  const kind = props.kind ?? "text";
  const disabled = props.disabled ?? false;
  return {
    label: props.label,
    labelling: props.labelling ?? "above",
    prefix: props.prefix,
    suffix: props.suffix,
    mono: props.mono ?? false,
    disabled,
    refused: props.error !== undefined,
    labelWire: { for: box },
    box: {
      id: box,
      type: kind,
      inputmode: kind === "number" ? "numeric" : undefined,
      step: props.step,
      pattern: props.pattern,
      value: props.value,
      placeholder: props.placeholder,
      "aria-disabled": disabled,
      "aria-invalid": props.error !== undefined,
      "aria-describedby": describedOf(props, note),
      oninput: (event) => {
        if (disabled) {
          event.currentTarget.value = props.value;
          return;
        }
        props.onInput(event.currentTarget.value);
      },
    },
    note: noteOf(props, note),
  };
}
