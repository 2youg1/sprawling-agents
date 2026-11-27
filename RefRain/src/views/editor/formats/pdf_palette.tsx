// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export function PdfToneFilter(props: { readonly id: string }) {
  return (
    <svg aria-hidden="true" class="pdf-tone-map" width="0" height="0">
      <defs>
        <filter
          id={props.id}
          x="0"
          y="0"
          width="100%"
          height="100%"
          color-interpolation-filters="sRGB"
        >
          <feColorMatrix
            in="SourceGraphic"
            type="matrix"
            values="0 0 0 0 0   0 0 0 0 0   0 0 0 0 0   -0.2126 -0.7152 -0.0722 0 1"
            result="coverage"
          />
          <feFlood class="pdf-tone-paper" result="paper" />
          <feFlood class="pdf-tone-ink" result="ink" />
          <feComposite in="ink" in2="coverage" operator="in" />
          <feComposite in2="paper" operator="over" />
        </filter>
      </defs>
    </svg>
  );
}
