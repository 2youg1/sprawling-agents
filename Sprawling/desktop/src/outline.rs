// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![cfg_attr(
    not(windows),
    expect(
        dead_code,
        reason = "the non-Windows arm reads no tree, so nothing folds one"
    )
)]

pub(crate) const LABEL_MOST: usize = 80;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Line<'a> {
    pub(crate) reference: &'a str,
    pub(crate) role: Role,
    pub(crate) name: &'a str,
    pub(crate) depth: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Ending {

    Whole,

    RefLimit { most: usize },

    Fault(String),
}

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
