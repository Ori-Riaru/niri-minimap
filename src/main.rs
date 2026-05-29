use niri_ipc::{Event, Request, Window, Workspace, socket::Socket};
use std::collections::BTreeMap;
use std::fmt::Display;

type OutputList = BTreeMap<String, WorkspaceList>;
type WorkspaceList = BTreeMap<u64, ColumnList>;
type ColumnList = BTreeMap<u64, WindowList>;
type WindowList = BTreeMap<u64, Window>;

struct NiriState {
    windows: BTreeMap<u64, Window>,
    workspaces: BTreeMap<u64, Workspace>,
}

impl NiriState {
    fn new() -> Self {
        NiriState {
            windows: BTreeMap::default(),
            workspaces: BTreeMap::default(),
        }
    }

    fn track_events(&mut self) {
        let mut socket = Socket::connect().unwrap();
        let _reply = socket.send(Request::EventStream).unwrap().unwrap();

        let mut read_event = socket.read_events();
        while let Ok(event) = read_event() {
            match event {
                // --- Workspaces ---
                Event::WorkspacesChanged {
                    workspaces: new_workspaces,
                } => {
                    self.workspaces = BTreeMap::<u64, Workspace>::default();
                    for workspace in new_workspaces {
                        self.workspaces.insert(workspace.id, workspace);
                    }
                }

                Event::WorkspaceUrgencyChanged { id, urgent } => {
                    if let Some(workspace) = self.workspaces.get_mut(&id) {
                        workspace.is_urgent = urgent;
                    };
                }

                Event::WorkspaceActivated { id, focused } => {
                    let active_output;
                    if let Some(workspace) = self.workspaces.get(&id) {
                        active_output = workspace.output.clone()
                    } else {
                        continue;
                    }

                    for workspace in self.workspaces.values_mut() {
                        if workspace.output == active_output {
                            workspace.is_active = false;
                        }
                        if focused {
                            workspace.is_focused = false;
                        }
                    }

                    if let Some(activated_workspace) = self.workspaces.get_mut(&id) {
                        activated_workspace.is_active = true;
                        activated_workspace.is_focused = true;
                    }
                }

                Event::WorkspaceActiveWindowChanged {
                    workspace_id,
                    active_window_id,
                } => {
                    if let Some(workspace) = self.workspaces.get_mut(&workspace_id) {
                        workspace.active_window_id = active_window_id;
                    }
                }

                // --- Windows ---
                Event::WindowsChanged {
                    windows: new_windows,
                } => {
                    self.windows = BTreeMap::<u64, Window>::new();
                    for window in new_windows {
                        self.windows.insert(window.id, window);
                    }
                }

                Event::WindowOpenedOrChanged { window } => {
                    if window.is_focused {
                        for window in self.windows.values_mut() {
                            window.is_focused = false;
                        }
                    }
                    self.windows.insert(window.id, window);
                }

                Event::WindowClosed { id } => {
                    self.windows.remove(&id);
                }

                Event::WindowFocusChanged { id } => {
                    for window in self.windows.values_mut() {
                        window.is_focused = false;
                    }

                    if let Some(id) = id {
                        if let Some(window) = self.windows.get_mut(&id) {
                            window.is_focused = true;
                        }
                    }
                }

                Event::WindowFocusTimestampChanged {
                    id,
                    focus_timestamp,
                } => {
                    if let Some(window) = self.windows.get_mut(&id) {
                        window.focus_timestamp = focus_timestamp;
                    }
                }

                Event::WindowUrgencyChanged { id, urgent } => {
                    if let Some(window) = self.windows.get_mut(&id) {
                        window.is_urgent = urgent;
                    }
                }

                Event::WindowLayoutsChanged { changes } => {
                    for (id, layout) in changes {
                        if let Some(window) = self.windows.get_mut(&id) {
                            window.layout = layout
                        }
                    }
                }
                _ => (),
            }

            // Render changes
            println!("{}", self);
        }
    }
}

impl Display for NiriState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Parse layout in to a hierarchical structure that is easier to render
        let mut outputs = OutputList::new();

        for workspace in self.workspaces.values() {
            let Some(output) = workspace.output.clone() else {
                continue;
            };
            outputs
                .entry(output)
                .or_insert_with(WorkspaceList::new)
                .insert(workspace.id, ColumnList::new());
        }

        let mut floating = WindowList::new();
        let workspaces = outputs.get_mut("DP-3").unwrap();

        for window in self.windows.values() {
            let Some((column_id, row_id)) = window.layout.pos_in_scrolling_layout else {
                floating.insert(
                    window.layout.window_offset_in_tile.0.floor() as u64,
                    window.clone(),
                );
                continue;
            };

            let Some(workspace_id) = window.workspace_id else {
                continue;
            };
            let Some(columns) = workspaces.get_mut(&workspace_id) else {
                continue;
            };
            let windows = columns
                .entry(column_id as u64)
                .or_insert_with(WindowList::new);

            windows.insert(row_id as u64, window.clone());
        }

        // Generate the resulting eww markdown
        let mut result =
            "(box :class 'minimap' :orientation 'vertical' :space-evenly false ".to_string();
        for workspace in workspaces.values() {
            if workspace.len() != 0 {
                result += "(box :class 'divider') (box :class 'workspace' :orientation 'vertical' :space-evenly false ";
                for column in workspace.values() {
                    result += "(scroll :vscroll false (box :class 'column' :space-evenly false :hexpand false :orientation 'horizontal'";
                    for window in column.values() {
                        let width = (window.layout.tile_size.1 / 50.0) as u32;
                        let height = (window.layout.tile_size.0 / 40.0) as u32;
                        let app_id = window.app_id.clone().unwrap_or("".into());
                        let window_id = window.id;
                        let focused_class = if window.is_focused { "focused" } else { "" };
                        let urgent_class = if window.is_urgent { "urgent" } else { "" };
                        let icon = window.app_id.clone().unwrap_or("".into());
                        let tooltip = window
                            .title
                            .clone()
                            .unwrap_or("".into())
                            .replace('\'', "\\'"); // FIXME do better escaping

                        result += format!(
                            "(button :class 'window {app_id} {urgent_class} {focused_class}' :height {height} :width {width} :tooltip '{tooltip}' :onclick 'niri msg action focus-window --id {window_id}' :onmiddleclick 'niri msg action close-window --id {window_id}' (scroll (image :icon '{icon}')))"
                        ).as_str();
                    }
                    result += "))"
                }
                result += ")"
            }
        }
        result += ")";

        write!(f, "{}", result.as_str())
    }
}

fn main() {
    let mut niri_state = NiriState::new();
    niri_state.track_events();
}
