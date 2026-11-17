// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use desktop_ffi::uia::{Automation, Element, Walker};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Accessibility::*;
use winsafe::co;
use winsafe::guard::CoUninitializeGuard;

use super::fault;
use super::geometry::Bounds;
use super::views::{MOST_REFS_PER_SNAPSHOT, Node};
use crate::outline::{self, Ending, Role};
use crate::refusal::Refusal;

pub(crate) struct Reader {
    walker: Walker,
    automation: Automation,
    _apartment: CoUninitializeGuard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Walked {
    pub(crate) nodes: Vec<Node>,
    pub(crate) ending: Ending,
}

impl Reader {
    pub(crate) fn start() -> Result<Reader, Refusal> {
        let apartment = winsafe::CoInitializeEx(co::COINIT::MULTITHREADED).map_err(|err| {
            fault::com(
                "join this thread to the desktop's automation apartment",
                "restart this server; a thread that cannot enter an apartment cannot read any \
                 tree",
                err,
            )
        })?;
        let automation = Automation::direct().map_err(|err| {
            fault::automation(
                "start UI Automation on this machine",
                "take a `desktop.screenshot` of the window instead; this machine's UI \
                 Automation service is not answering",
                &err,
            )
        })?;
        let walker = automation.control_view_walker().map_err(|err| {
            fault::automation(
                "open a walk over the window's tree",
                "take a `desktop.screenshot` of this window instead",
                &err,
            )
        })?;
        Ok(Reader {
            walker,
            automation,
            _apartment: apartment,
        })
    }

    pub(crate) fn read(&self, window: &winsafe::HWND, depth: u32) -> Result<Walked, Refusal> {
        let root = self
            .automation
            .element_from_handle(HWND(window.ptr()))
            .map_err(|err| {
                fault::automation(
                    "read the window's accessibility tree",
                    "call `desktop.windows` again; a window that has closed has no tree, and a \
                 window drawn without accessibility information cannot be read this way — use \
                 `desktop.screenshot` to see it instead",
                    &err,
                )
            })?;
        Ok(walk(self, root, depth))
    }
}

pub(crate) trait Branches {
    type Element;

    fn seen(&self, element: &Self::Element) -> Option<Seen>;

    fn children(&self, parent: &Self::Element) -> Result<Vec<Self::Element>, String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Seen {
    pub(crate) role: Role,
    pub(crate) name: String,
    pub(crate) bounds: Bounds,
}

impl Branches for Reader {
    type Element = Element;

    fn seen(&self, element: &Element) -> Option<Seen> {
        let rect = element.bounds().ok()?;
        Some(Seen {
            role: role(element.control_type().ok()?)?,
            name: element.name().ok()?,
            bounds: Bounds::from_corners(rect.left, rect.top, rect.right, rect.bottom).ok()?,
        })
    }

    fn children(&self, parent: &Element) -> Result<Vec<Element>, String> {
        let mut found: Vec<Element> = Vec::new();
        let mut next = listed(self.walker.first_child(parent))?;
        while let Some(child) = next {
            if found.len() >= MOST_REFS_PER_SNAPSHOT {
                break;
            }
            next = listed(self.walker.next_sibling(&child))?;
            found.push(child);
        }
        Ok(found)
    }
}

fn listed(step: Result<Option<Element>, windows::core::Error>) -> Result<Option<Element>, String> {
    step.map_err(|err| format!("an element would not list what it holds ({err})"))
}

pub(crate) fn walk<T: Branches>(tree: &T, root: T::Element, depth: u32) -> Walked {
    let mut nodes: Vec<Node> = Vec::new();
    let mut frontier: Vec<(T::Element, u32)> = vec![(root, 0)];
    while let Some((element, level)) = frontier.pop() {
        if nodes.len() >= MOST_REFS_PER_SNAPSHOT {
            return Walked {
                nodes,
                ending: Ending::RefLimit {
                    most: MOST_REFS_PER_SNAPSHOT,
                },
            };
        }
        if let Some(seen) = tree.seen(&element) {
            let name = outline::label(&seen.name);
            if !(name.is_empty() && seen.role.frames()) {
                nodes.push(Node {
                    reference: format!("e{}", nodes.len().saturating_add(1)),
                    role: seen.role,
                    name,
                    bounds: seen.bounds,
                    depth: level,
                });
            }
        }
        if level >= depth {
            continue;
        }
        match tree.children(&element) {
            Ok(children) => frontier.extend(
                children
                    .into_iter()
                    .rev()
                    .map(|child| (child, level.saturating_add(1))),
            ),
            Err(why) => {
                return Walked {
                    nodes,
                    ending: Ending::Fault(why),
                };
            }
        }
    }
    Walked {
        nodes,
        ending: Ending::Whole,
    }
}

#[expect(
    non_upper_case_globals,
    reason = "the UIA catalog spells its control type constants in camelCase; matching \
              them by name keeps this table greppable against that catalog"
)]
fn role(control: UIA_CONTROLTYPE_ID) -> Option<Role> {
    match control {
        UIA_ButtonControlTypeId => Some(Role::Button),
        UIA_CalendarControlTypeId => Some(Role::Calendar),
        UIA_CheckBoxControlTypeId => Some(Role::CheckBox),
        UIA_ComboBoxControlTypeId => Some(Role::ComboBox),
        UIA_EditControlTypeId => Some(Role::Edit),
        UIA_HyperlinkControlTypeId => Some(Role::Hyperlink),
        UIA_ImageControlTypeId => Some(Role::Image),
        UIA_ListItemControlTypeId => Some(Role::ListItem),
        UIA_ListControlTypeId => Some(Role::List),
        UIA_MenuControlTypeId => Some(Role::Menu),
        UIA_MenuBarControlTypeId => Some(Role::MenuBar),
        UIA_MenuItemControlTypeId => Some(Role::MenuItem),
        UIA_ProgressBarControlTypeId => Some(Role::ProgressBar),
        UIA_RadioButtonControlTypeId => Some(Role::RadioButton),
        UIA_ScrollBarControlTypeId => Some(Role::ScrollBar),
        UIA_SliderControlTypeId => Some(Role::Slider),
        UIA_SpinnerControlTypeId => Some(Role::Spinner),
        UIA_StatusBarControlTypeId => Some(Role::StatusBar),
        UIA_TabControlTypeId => Some(Role::Tab),
        UIA_TabItemControlTypeId => Some(Role::TabItem),
        UIA_TextControlTypeId => Some(Role::Text),
        UIA_ToolBarControlTypeId => Some(Role::ToolBar),
        UIA_ToolTipControlTypeId => Some(Role::ToolTip),
        UIA_TreeControlTypeId => Some(Role::Tree),
        UIA_TreeItemControlTypeId => Some(Role::TreeItem),
        UIA_CustomControlTypeId => Some(Role::Custom),
        UIA_GroupControlTypeId => Some(Role::Group),
        UIA_ThumbControlTypeId => Some(Role::Thumb),
        UIA_DataGridControlTypeId => Some(Role::DataGrid),
        UIA_DataItemControlTypeId => Some(Role::DataItem),
        UIA_DocumentControlTypeId => Some(Role::Document),
        UIA_SplitButtonControlTypeId => Some(Role::SplitButton),
        UIA_WindowControlTypeId => Some(Role::Window),
        UIA_PaneControlTypeId => Some(Role::Pane),
        UIA_HeaderControlTypeId => Some(Role::Header),
        UIA_HeaderItemControlTypeId => Some(Role::HeaderItem),
        UIA_TableControlTypeId => Some(Role::Table),
        UIA_TitleBarControlTypeId => Some(Role::TitleBar),
        UIA_SeparatorControlTypeId => Some(Role::Separator),
        UIA_SemanticZoomControlTypeId => Some(Role::SemanticZoom),
        UIA_AppBarControlTypeId => Some(Role::AppBar),
        _ => None,
    }
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
    use super::super::enumerate;
    use super::super::fixture::Opened;
    use super::*;
    use std::collections::BTreeMap;
    use winsafe::prelude::Handle as _;

    struct Toy {
        said: BTreeMap<u32, (Role, &'static str)>,
        holds: BTreeMap<u32, Result<Vec<u32>, String>>,
    }

    impl Branches for Toy {
        type Element = u32;

        fn seen(&self, element: &u32) -> Option<Seen> {
            let (role, name) = self.said.get(element)?;
            Some(Seen {
                role: *role,
                name: (*name).to_owned(),
                bounds: Bounds::from_corners(0, 0, 10, 10).unwrap(),
            })
        }

        fn children(&self, parent: &u32) -> Result<Vec<u32>, String> {
            self.holds.get(parent).cloned().unwrap_or(Ok(Vec::new()))
        }
    }

    fn toy(said: &[(u32, Role, &'static str)], holds: Vec<(u32, Result<Vec<u32>, String>)>) -> Toy {
        Toy {
            said: said
                .iter()
                .map(|(element, role, name)| (*element, (*role, *name)))
                .collect(),
            holds: holds.into_iter().collect(),
        }
    }

    fn minted(walked: &Walked) -> (Vec<(&str, &str, u32)>, &Ending) {
        (
            walked
                .nodes
                .iter()
                .map(|node| (node.reference.as_str(), node.name.as_str(), node.depth))
                .collect(),
            &walked.ending,
        )
    }

    #[test]
    fn siblings_are_minted_in_the_order_the_window_lists_them() {
        let tree = toy(
            &[
                (0, Role::Window, "w"),
                (1, Role::Group, "a"),
                (2, Role::Button, "a1"),
                (3, Role::Button, "a2"),
                (4, Role::Button, "b"),
            ],
            vec![(0, Ok(vec![1, 4])), (1, Ok(vec![2, 3]))],
        );
        let walked = walk(&tree, 0, 8);
        assert_eq!(
            minted(&walked),
            (
                vec![
                    ("e1", "w", 0),
                    ("e2", "a", 1),
                    ("e3", "a1", 2),
                    ("e4", "a2", 2),
                    ("e5", "b", 1),
                ],
                &Ending::Whole
            )
        );
    }

    #[test]
    fn unnamed_frames_cost_no_line_and_hide_nothing() {
        let tree = toy(
            &[
                (0, Role::Window, "w"),
                (1, Role::Pane, ""),
                (2, Role::Button, "ok"),
            ],
            vec![(0, Ok(vec![1])), (1, Ok(vec![2]))],
        );
        assert_eq!(
            minted(&walk(&tree, 0, 8)),
            (vec![("e1", "w", 0), ("e2", "ok", 2)], &Ending::Whole)
        );
    }

    #[test]
    fn a_walker_fault_ends_the_walk_and_says_so() {
        let tree = toy(
            &[
                (0, Role::Window, "w"),
                (1, Role::List, "l"),
                (2, Role::Button, "b"),
            ],
            vec![
                (0, Ok(vec![1, 2])),
                (1, Err("the element is gone".to_owned())),
            ],
        );
        assert_eq!(
            minted(&walk(&tree, 0, 8)),
            (
                vec![("e1", "w", 0), ("e2", "l", 1)],
                &Ending::Fault("the element is gone".to_owned())
            )
        );
    }

    #[test]
    fn a_tree_past_the_ref_limit_says_so() {
        let buttons: Vec<u32> = (1..=600).collect();
        let mut said = vec![(0, Role::Window, "w")];
        said.extend(buttons.iter().map(|button| (*button, Role::Button, "x")));
        let tree = toy(&said, vec![(0, Ok(buttons))]);
        let walked = walk(&tree, 0, 8);
        assert_eq!(
            (walked.nodes.len(), walked.ending),
            (
                MOST_REFS_PER_SNAPSHOT,
                Ending::RefLimit {
                    most: MOST_REFS_PER_SNAPSHOT
                }
            )
        );
    }

    #[test]
    fn a_windows_tree_names_the_control_inside_it() {
        let title = format!("sprawling contract tree {}", std::process::id());
        let _opened = Opened::at(&title, -20_000, -20_000, Some("Press"));
        let window = enumerate::desktop()
            .unwrap()
            .into_iter()
            .find(|window| window.named.title == title)
            .unwrap();
        let walked = Reader::start().unwrap().read(&window.handle, 2).unwrap();
        let named: Vec<(&str, u32)> = walked
            .nodes
            .iter()
            .map(|node| (node.name.as_str(), node.depth))
            .collect();
        assert_eq!(named, vec![(title.as_str(), 0), ("Press", 1)]);
    }

    #[test]
    fn a_handle_that_names_no_window_is_refused_with_a_next_step() {
        let refusal = Reader::start()
            .unwrap()
            .read(&winsafe::HWND::NULL, 2)
            .expect_err("no window, no tree, and no pretending otherwise");
        let error = refusal.as_error();
        assert_eq!(error["data"]["code"], "E_TOOL_UNAVAILABLE");
        assert!(
            error["data"]["recovery"]
                .as_str()
                .unwrap()
                .contains("desktop.screenshot")
        );
    }

    #[test]
    fn two_readers_on_one_thread_both_start() {
        let first = Reader::start();
        let second = Reader::start();
        assert!(first.is_ok() && second.is_ok());
    }
}
