// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

pub(crate) const BOX_OF: &str = "function boxOf(node) {\n  \
    var rect = node.getBoundingClientRect();\n  \
    return [Math.round(rect.left), Math.round(rect.top), Math.round(rect.width), \
    Math.round(rect.height)];\n\
    }";
