// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { lazy, Loading } from "solid-js";
import type { PdfProps } from "./pdf_view";

const Pdf = lazy(() => import("./pdf_view").then(({ PdfView }) => ({ default: PdfView })));

export function PdfLoading(props: PdfProps) {
  return (
    <Loading fallback={null}>
      <Pdf {...props} />
    </Loading>
  );
}
