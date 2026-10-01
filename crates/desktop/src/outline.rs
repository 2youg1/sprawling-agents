// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How one window's tree reads to a model.
//!
//! A model acting on a window needs to know what is in it and what to
//! call each thing, and nothing else: not where a control sits on the
//! screen, which `desktop.act` looks up by ref, and not the nesting of
//! containers nobody named. So a tree crosses as an outline, one line
//! per element — its ref, its role, its name — indented by depth
//! (`crates/desktop/Spec.lean` D10 (b)).
//!
//! The role is a word from a closed vocabulary rather than whatever the
//! platform says: a localized string names one button two ways on two
//! machines, and "everything the platform says" grows whenever the
//! platform does. The vocabulary is UI Automation's own control types,
//! spelled as their lower-case names, and it is platform-free so that a
//! second platform maps onto the same words.
//!
//! A name is the window author's text, so it is cleaned and cut before
//! it crosses: a control character would break the line it sits on, and
//! a name as long as a page is a page rather than a name.
//!
//! The properties this module must hold are proved in `crates/desktop/spec/Outline.lean`;
//! this code is the authority on how it holds them.

// The tests run everywhere, but only the Windows arm reads a tree, so
// elsewhere most of the vocabulary is never spoken, in tests included.
#![cfg_attr(
    not(windows),
    expect(
        dead_code,
        reason = "the non-Windows arm reads no tree, so nothing folds one"
    )
)]

/// How many characters of a name survive. Long enough to tell two
/// controls apart, short enough that a window's outline is still a
/// page.
pub(crate) const LABEL_MOST: usize = 80;

/// What an element is, in the words a model reads.
///
/// One variant per UI Automation control type, so the mapping from a
/// type is a match with no fallback and a type the platform adds is a
/// compile error rather than a silent default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    Button,
    Calendar,
    CheckBox,
    ComboBox,
    Edit,
    Hyperlink,
    Image,
    ListItem,
    List,
    Menu,
    MenuBar,
    MenuItem,
    ProgressBar,
    RadioButton,
    ScrollBar,
    Slider,
    Spinner,
    StatusBar,
    Tab,
    TabItem,
    Text,
    ToolBar,
    ToolTip,
    Tree,
    TreeItem,
    Custom,
    Group,
    Thumb,
    DataGrid,
    DataItem,
    Document,
    SplitButton,
    Window,
    Pane,
    Header,
    HeaderItem,
    Table,
    TitleBar,
    Separator,
    SemanticZoom,
    AppBar,
}

impl Role {
    /// The word this role is written as.
    pub(crate) const fn word(self) -> &'static str {
        match self {
            Role::Button => "button",
            Role::Calendar => "calendar",
            Role::CheckBox => "checkbox",
            Role::ComboBox => "combobox",
            Role::Edit => "edit",
            Role::Hyperlink => "hyperlink",
            Role::Image => "image",
            Role::ListItem => "listitem",
            Role::List => "list",
            Role::Menu => "menu",
            Role::MenuBar => "menubar",
            Role::MenuItem => "menuitem",
            Role::ProgressBar => "progressbar",
            Role::RadioButton => "radiobutton",
            Role::ScrollBar => "scrollbar",
            Role::Slider => "slider",
            Role::Spinner => "spinner",
            Role::StatusBar => "statusbar",
            Role::Tab => "tab",
            Role::TabItem => "tabitem",
            Role::Text => "text",
            Role::ToolBar => "toolbar",
            Role::ToolTip => "tooltip",
            Role::Tree => "tree",
            Role::TreeItem => "treeitem",
            Role::Custom => "custom",
            Role::Group => "group",
            Role::Thumb => "thumb",
            Role::DataGrid => "datagrid",
            Role::DataItem => "dataitem",
            Role::Document => "document",
            Role::SplitButton => "splitbutton",
            Role::Window => "window",
            Role::Pane => "pane",
            Role::Header => "header",
            Role::HeaderItem => "headeritem",
            Role::Table => "table",
            Role::TitleBar => "titlebar",
            Role::Separator => "separator",
            Role::SemanticZoom => "semanticzoom",
            Role::AppBar => "appbar",
        }
    }

    /// Whether an element of this role only frames others. A frame
    /// nobody named is not worth a line or a ref, and what it holds is
    /// read as if it were not there.
    pub(crate) const fn frames(self) -> bool {
        match self {
            Role::Pane | Role::Group | Role::Custom | Role::Separator => true,
            Role::Button
            | Role::Calendar
            | Role::CheckBox
            | Role::ComboBox
            | Role::Edit
            | Role::Hyperlink
            | Role::Image
            | Role::ListItem
            | Role::List
            | Role::Menu
            | Role::MenuBar
            | Role::MenuItem
            | Role::ProgressBar
            | Role::RadioButton
            | Role::ScrollBar
            | Role::Slider
            | Role::Spinner
            | Role::StatusBar
            | Role::Tab
            | Role::TabItem
            | Role::Text
            | Role::ToolBar
            | Role::ToolTip
            | Role::Tree
            | Role::TreeItem
            | Role::Thumb
            | Role::DataGrid
            | Role::DataItem
            | Role::Document
            | Role::SplitButton
            | Role::Window
            | Role::Header
            | Role::HeaderItem
            | Role::Table
            | Role::TitleBar
            | Role::SemanticZoom
            | Role::AppBar => false,
        }
    }
}

/// One line of an outline: an element as a model reads it. `name` has
/// already been through [`label`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Line<'a> {
    pub(crate) reference: &'a str,
    pub(crate) role: Role,
    pub(crate) name: &'a str,
    pub(crate) depth: u32,
}

/// Why a walk over a tree stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Ending {
    /// Everything down to the depth asked for was read.
    Whole,
    /// The snapshot minted the most refs one snapshot mints.
    RefLimit { most: usize },
    /// An element would not list what it holds; the words are the
    /// platform's.
    Fault(String),
}

/// A name as it crosses: control characters become spaces, runs of
/// white space become one, and a name past [`LABEL_MOST`] characters is
/// cut there and ends in `…`.
pub(crate) fn label(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect();
    let joined = cleaned.split_whitespace().collect::<Vec<&str>>().join(" ");
    if joined.chars().count() <= LABEL_MOST {
        return joined;
    }
    let mut cut: String = joined.chars().take(LABEL_MOST).collect();
    cut.push('…');
    cut
}

/// The outline: one line per element, indented two spaces per level of
/// depth, and a last line saying why the walk stopped when it stopped
/// early. The same lines always fold to the same bytes.
pub(crate) fn fold<'a>(lines: impl IntoIterator<Item = Line<'a>>, ending: &Ending) -> String {
    let mut out = String::new();
    for line in lines {
        for _level in 0..line.depth {
            out.push_str("  ");
        }
        out.push_str(line.reference);
        out.push(' ');
        out.push_str(line.role.word());
        if !line.name.is_empty() {
            out.push_str(" \"");
            out.push_str(line.name);
            out.push('"');
        }
        out.push('\n');
    }
    match ending {
        Ending::Whole => {}
        Ending::RefLimit { most } => {
            out.push_str(&format!(
                "… stopped at {most} refs, the most one snapshot mints\n"
            ));
        }
        Ending::Fault(why) => {
            out.push_str(&format!(
                "… stopped early: {why}; nothing below this line was read\n"
            ));
        }
    }
    out
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    fn line(reference: &'static str, role: Role, name: &'static str, depth: u32) -> Line<'static> {
        Line {
            reference,
            role,
            name,
            depth,
        }
    }

    /// The shape a model reads, byte for byte: a ref, a role word and a
    /// quoted name, two spaces of indent per level, and no quotes where
    /// there is no name. Folding twice gives the same bytes.
    #[test]
    fn a_window_folds_to_the_same_text_every_time() {
        let lines = [
            line("e1", Role::Window, "Untitled - Notepad", 0),
            line("e2", Role::MenuBar, "Application", 1),
            line("e3", Role::MenuItem, "File", 2),
            line("e4", Role::Edit, "", 1),
        ];
        let expected = "e1 window \"Untitled - Notepad\"\n  e2 menubar \"Application\"\n    e3 \
                        menuitem \"File\"\n  e4 edit\n";
        assert_eq!(fold(lines, &Ending::Whole), expected);
        assert_eq!(fold(lines, &Ending::Whole), fold(lines, &Ending::Whole));
    }

    /// A walk that stopped early says so in its last line, and says why:
    /// an outline that ends quietly reads as a window that ends there.
    #[test]
    fn an_outline_cut_short_says_where_and_why() {
        let lines = [line("e1", Role::Window, "w", 0)];
        assert_eq!(
            fold(lines, &Ending::RefLimit { most: 500 }),
            "e1 window \"w\"\n… stopped at 500 refs, the most one snapshot mints\n"
        );
        assert_eq!(
            fold(lines, &Ending::Fault("the element is gone".to_owned())),
            "e1 window \"w\"\n… stopped early: the element is gone; nothing below this line \
             was read\n"
        );
    }

    /// A name is the window author's text: a newline inside it would
    /// break the outline, and a page of it is not a name.
    #[test]
    fn a_label_with_control_characters_and_a_long_tail_is_cleaned_and_cut() {
        assert_eq!(label("Save\nas\t\u{7}  copy "), "Save as copy");
        let long = "x".repeat(LABEL_MOST + 5);
        let cut = label(&long);
        assert_eq!(cut.chars().count(), LABEL_MOST + 1);
        assert!(cut.ends_with('…'));
        assert_eq!(label(&"x".repeat(LABEL_MOST)), "x".repeat(LABEL_MOST));
    }
}
