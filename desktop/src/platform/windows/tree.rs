// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The UI Automation tree of one window: role, name, ref and bounds,
//! read through `uiautomation`'s safe calls (desktop-SPEC.md section
//! 12.10).
//!
//! **Every element this module reads dies inside the call that read
//! it.** A `ref` handed back to a caller is a rectangle and a
//! generation, never a live element (desktop-SPEC.md §8.6, second pair),
//! so a connection that snapshots a thousand windows holds a thousand
//! rectangles rather than a thousand cross-process interface pointers.
//! What does outlive a call is the [`Reader`]: the COM apartment, the
//! automation object and the walker one desk reads every tree with,
//! entered once and left once.
//!
//! The walk is bounded twice — by the depth the caller asked for and by
//! [`views::MOST_REFS_PER_SNAPSHOT`] — because a UI Automation tree has
//! no promise of being finite in any practical sense: one list control
//! can present tens of thousands of items, and a snapshot a model
//! cannot read to the end is a snapshot it has not read.
//!
//! The walk itself is a function over any tree that answers
//! [`Branches`], so the order it mints refs in and where it stops can be
//! checked on a tree a test writes down, with no window at all.
//!
//! [`views::MOST_REFS_PER_SNAPSHOT`]: super::views::MOST_REFS_PER_SNAPSHOT

use uiautomation::types::{ControlType, Handle};
use uiautomation::{UIAutomation, UIElement, UITreeWalker};
use winsafe::co;
use winsafe::guard::CoUninitializeGuard;

use super::fault;
use super::geometry::Bounds;
use super::views::{MOST_REFS_PER_SNAPSHOT, Node};
use crate::outline::{Ending, Role};
use crate::refusal::Refusal;

/// What one desk reads trees with.
///
/// The fields drop in the order they are declared, and that order is
/// the rule: the walker and the automation object are COM objects, so
/// they go before the apartment they live in is left.
pub(crate) struct Reader {
    walker: UITreeWalker,
    automation: UIAutomation,
    _apartment: CoUninitializeGuard,
}

/// One window's tree as a walk found it: the nodes it minted refs for,
/// and why it stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Walked {
    pub(crate) nodes: Vec<Node>,
    pub(crate) ending: Ending,
}

impl Reader {
    /// Enters this thread's apartment and starts UI Automation.
    ///
    /// The multithreaded apartment, because this thread opens no window
    /// that COM would have to pump messages for, which is the case
    /// Microsoft recommends it for (desktop-SPEC.md section 12.10).
    ///
    /// # Errors
    /// Refuses when this thread cannot enter an apartment, and when UI
    /// Automation will not start or will not open a walk.
    pub(crate) fn start() -> Result<Reader, Refusal> {
        let apartment = winsafe::CoInitializeEx(co::COINIT::MULTITHREADED).map_err(|err| {
            fault::com(
                "join this thread to the desktop's automation apartment",
                "restart this server; a thread that cannot enter an apartment cannot read any \
                 tree",
                err,
            )
        })?;
        let automation = UIAutomation::new_direct().map_err(|err| {
            fault::automation(
                "start UI Automation on this machine",
                "take a `desktop.screenshot` of the window instead; this machine's UI \
                 Automation service is not answering",
                &err,
            )
        })?;
        let walker = automation.get_control_view_walker().map_err(|err| {
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

    /// Reads one window's tree.
    ///
    /// # Errors
    /// Refuses when the window has no automation element. An element
    /// that will not describe itself is skipped, because one unreadable
    /// control is not a reason to report no window; one that will not
    /// list what it holds ends the walk, which says so.
    pub(crate) fn read(&self, window: &winsafe::HWND, depth: u32) -> Result<Walked, Refusal> {
        let handle = Handle::from(windows::Win32::Foundation::HWND(window.ptr()));
        let root = self.automation.element_from_handle(handle).map_err(|err| {
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

/// A tree the walk can go over: what an element says of itself, and
/// what it holds. The UI Automation tree is one; a tree a test writes
/// down is the other.
pub(crate) trait Branches {
    type Element;

    /// The element's role, name and place, or `None` when it will not
    /// say them.
    fn seen(&self, element: &Self::Element) -> Option<Seen>;

    /// The elements `parent` holds, in the order the tree lists them.
    ///
    /// # Errors
    /// The platform's words when the element will not list them.
    fn children(&self, parent: &Self::Element) -> Result<Vec<Self::Element>, String>;
}

/// What one element says of itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Seen {
    pub(crate) role: Role,
    pub(crate) name: String,
    pub(crate) bounds: Bounds,
}

impl Branches for Reader {
    type Element = UIElement;

    fn seen(&self, element: &UIElement) -> Option<Seen> {
        let rect = element.get_bounding_rectangle().ok()?;
        Some(Seen {
            role: role(element.get_control_type().ok()?),
            name: element.get_name().ok()?,
            bounds: Bounds::from_corners(
                rect.get_left(),
                rect.get_top(),
                rect.get_right(),
                rect.get_bottom(),
            )
            .ok()?,
        })
    }

    fn children(&self, parent: &UIElement) -> Result<Vec<UIElement>, String> {
        let mut found: Vec<UIElement> = Vec::new();
        let mut next = self.walker.get_first_child(parent).ok();
        while let Some(child) = next {
            if found.len() >= MOST_REFS_PER_SNAPSHOT {
                break;
            }
            next = self.walker.get_next_sibling(&child).ok();
            found.push(child);
        }
        Ok(found)
    }
}

/// The bounded walk, and the only place a ref is minted.
pub(crate) fn walk<T: Branches>(tree: &T, root: T::Element, depth: u32) -> Walked {
    let mut nodes: Vec<Node> = Vec::new();
    let mut frontier: Vec<(T::Element, u32)> = vec![(root, 0)];
    while let Some((element, level)) = frontier.pop() {
        if nodes.len() >= MOST_REFS_PER_SNAPSHOT {
            break;
        }
        if let Some(seen) = tree.seen(&element) {
            nodes.push(Node {
                reference: format!("e{}", nodes.len().saturating_add(1)),
                role: seen.role,
                name: seen.name,
                bounds: seen.bounds,
                depth: level,
            });
        }
        if level >= depth {
            continue;
        }
        let Ok(children) = tree.children(&element) else {
            continue;
        };
        for child in children {
            frontier.push((child, level.saturating_add(1)));
        }
    }
    Walked {
        nodes,
        ending: Ending::Whole,
    }
}

/// The word a control type is. A match with no fallback, so a type the
/// binding adds is a compile error here rather than a silent default.
fn role(control: ControlType) -> Role {
    match control {
        ControlType::Button => Role::Button,
        ControlType::Calendar => Role::Calendar,
        ControlType::CheckBox => Role::CheckBox,
        ControlType::ComboBox => Role::ComboBox,
        ControlType::Edit => Role::Edit,
        ControlType::Hyperlink => Role::Hyperlink,
        ControlType::Image => Role::Image,
        ControlType::ListItem => Role::ListItem,
        ControlType::List => Role::List,
        ControlType::Menu => Role::Menu,
        ControlType::MenuBar => Role::MenuBar,
        ControlType::MenuItem => Role::MenuItem,
        ControlType::ProgressBar => Role::ProgressBar,
        ControlType::RadioButton => Role::RadioButton,
        ControlType::ScrollBar => Role::ScrollBar,
        ControlType::Slider => Role::Slider,
        ControlType::Spinner => Role::Spinner,
        ControlType::StatusBar => Role::StatusBar,
        ControlType::Tab => Role::Tab,
        ControlType::TabItem => Role::TabItem,
        ControlType::Text => Role::Text,
        ControlType::ToolBar => Role::ToolBar,
        ControlType::ToolTip => Role::ToolTip,
        ControlType::Tree => Role::Tree,
        ControlType::TreeItem => Role::TreeItem,
        ControlType::Custom => Role::Custom,
        ControlType::Group => Role::Group,
        ControlType::Thumb => Role::Thumb,
        ControlType::DataGrid => Role::DataGrid,
        ControlType::DataItem => Role::DataItem,
        ControlType::Document => Role::Document,
        ControlType::SplitButton => Role::SplitButton,
        ControlType::Window => Role::Window,
        ControlType::Pane => Role::Pane,
        ControlType::Header => Role::Header,
        ControlType::HeaderItem => Role::HeaderItem,
        ControlType::Table => Role::Table,
        ControlType::TitleBar => Role::TitleBar,
        ControlType::Separator => Role::Separator,
        ControlType::SemanticZoom => Role::SemanticZoom,
        ControlType::AppBar => Role::AppBar,
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

    /// A tree written down: each element a number, with what it says of
    /// itself and what it holds.
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

    /// The refs a walk minted, as (ref, name, depth), and why it stopped.
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

    /// Refs are minted in document order: a parent before what it holds,
    /// and siblings in the order the window lists them. A model reading
    /// the outline top to bottom reads the window top to bottom.
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

    /// A frame nobody named costs no line and no ref, and what it holds
    /// is read as if it were not there.
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

    /// An element that will not list what it holds ends the walk, and
    /// the walk says so rather than reading as a window that ends there.
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

    /// A tree with more elements than one snapshot mints refs for stops
    /// at the limit and says it did.
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

    /// The contract desktop-SPEC.md section 8-11 holds the tree to,
    /// whichever interface reads it: a window's tree starts at the window,
    /// named by its title, and names the control inside it one level
    /// down. Roles are not compared, because they are a word this server
    /// chooses rather than a fact the window reports.
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

    /// A window that is not there has no tree, and that is a refusal
    /// naming a next step rather than a crash. This is the one thing
    /// about this module that holds on a machine with no desktop
    /// (desktop-SPEC.md §16.2).
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

    /// Two readers on one thread are two entries into one apartment,
    /// each left when its reader goes: a connection that is served
    /// twice on one thread does not find the apartment already left.
    #[test]
    fn two_readers_on_one_thread_both_start() {
        let first = Reader::start();
        let second = Reader::start();
        assert!(first.is_ok() && second.is_ok());
    }
}
