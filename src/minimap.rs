use crate::config::Config;
use core::panic;
use niri_ipc::{Event, Output, Request, Response, Window, Workspace, socket::Socket};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Display;

type OutputMap = BTreeMap<String, WorkspaceMap>;

type WorkspaceMap = BTreeMap<u64, ColumnMap>;

#[derive(Default, Debug)]
struct ColumnMap {
    columns: BTreeMap<u64, WindowMap>,
    floating: BTreeMap<u64, Window>,
}

type WindowMap = BTreeMap<u64, Window>;

pub struct Minimap {
    outputs: HashMap<String, Output>,
    windows: BTreeMap<u64, Window>,
    workspaces: BTreeMap<u64, Workspace>,
    config: Config,
}

impl Minimap {
    pub fn new(config: Config) -> Self {
        let mut socket = Socket::connect().unwrap();
        let response = socket.send(Request::Outputs).unwrap().unwrap();
        let Response::Outputs(outputs) = response else {
            panic!("Failed to get Niri outputs")
        };

        Minimap {
            outputs,
            windows: BTreeMap::default(),
            workspaces: BTreeMap::default(),
            config,
        }
    }

    pub fn track_events(&mut self) {
        let mut socket = Socket::connect().unwrap(); // FIXME: unwrap
        let _reply = socket.send(Request::EventStream).unwrap().unwrap(); // FIXME: unwrap

        let mut read_event = socket.read_events();
        while let Ok(event) = read_event() {
            // === Update State ===

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

            // === Render changes ===

            println!("{}", self);
        }
    }

    fn render_window(&self, window: &Window, scale: f64, floating: bool) -> String {
        let mut result = String::new();
        let size;
        if !floating {
            let width = (window.layout.tile_size.1 / scale) as u32;
            let height = (window.layout.tile_size.0 / scale * self.config.aspect_ratio) as u32;

            size = format!(":width {width} :height {height}");
        } else {
            size = ":height 30".into();
        }

        let window_id = window.id;
        let focused_class = if window.is_focused { "focused" } else { "" };
        let urgent_class = if window.is_urgent { "urgent" } else { "" };
        let app_class = window.app_id.clone().unwrap_or("".into()).replace(".", "-");
        let icon = window.app_id.clone().unwrap_or("".into());
        let tooltip = window
            .title
            .clone()
            .unwrap_or("".into())
            .replace('\'', "\\'"); // FIXME do better escaping

        result += format!(
        "(button :class 'window {app_class} {urgent_class} {focused_class}' {size} :tooltip '{tooltip}' :onclick 'niri msg action focus-window --id {window_id}' :onmiddleclick 'niri msg action close-window --id {window_id}' (scroll (image :icon '{icon}')))"
    ).as_str();

        result
    }
}

impl Display for Minimap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Parse layout in to a hierarchical structure that is easier to render
        let mut outputs = OutputMap::default();

        for window in self.windows.values() {
            let Some(workspace_id) = window.workspace_id else {
                continue;
            };
            let Some(workspace) = self.workspaces.get(&workspace_id) else {
                continue;
            };
            let Some(output_id) = workspace.output.clone() else {
                continue;
            };

            let output = self.outputs.get(&output_id).unwrap(); // FIXME: unwrap
            let mut output_full_name = format!("{} {}", output.make, output.model);
            if let Some(serial) = output.serial.clone() {
                output_full_name = format!("{} {}", output_full_name, serial)
            };

            if let Some(target_outputs) = self.config.output.clone() {
                let a_target = target_outputs.iter().any(|target_output| {
                    *target_output == *output_id
                        || Some(target_output.clone()) == output.serial
                        || *target_output == *output_full_name
                });
                if !a_target {
                    continue;
                }
            }

            if let Some((column_id, row_id)) = window.layout.pos_in_scrolling_layout {
                outputs
                    .entry(output_id)
                    .or_insert_with(WorkspaceMap::default)
                    .entry(workspace_id)
                    .or_insert_with(ColumnMap::default)
                    .columns
                    .entry(column_id as u64)
                    .or_insert_with(WindowMap::default)
                    .insert(row_id as u64, window.clone());
            } else {
                let Some((x, _y)) = window.layout.tile_pos_in_workspace_view else {
                    continue;
                };

                outputs
                    .entry(output_id)
                    .or_insert_with(WorkspaceMap::default)
                    .entry(workspace_id)
                    .or_insert_with(ColumnMap::default)
                    .floating
                    .insert(x as u64, window.clone());
            }
        }

        // Generate the resulting eww markdown
        let mut result =
            "(box :class 'minimap' :orientation 'vertical' :space-evenly false ".to_string();
        for (output_key, output) in outputs.iter() {
            result += "(box :class 'output' :orientation 'vertical' :space-evenly false ";

            let o = self.outputs.get(output_key).unwrap(); // FIXME: unwrap
            let logical_output: niri_ipc::LogicalOutput = o.logical.unwrap(); // FIXME: unwrap
            let scale = logical_output.height as f64 / self.config.height as f64;

            for workspace in output.values() {
                if workspace.columns.len() != 0 {
                    result += "(box :class 'divider') (box :class 'workspace' :orientation 'vertical' :space-evenly false ";
                    for column in workspace.columns.values() {
                        result += "(eventbox :cursor 'pointer' (scroll :vscroll false (box :class 'column' :space-evenly false :hexpand false :orientation 'horizontal'";
                        for window in column.values() {
                            result += self.render_window(window, scale, false).as_str();
                        }
                        result += ")))"
                    }
                    result += ")";
                }

                if workspace.floating.len() != 0 {
                    result += "(eventbox :cursor 'pointer' (scroll :vscroll false (box :class 'column' :space-evenly true :hexpand false :orientation 'horizontal'";
                    for window in workspace.floating.values() {
                        result += self.render_window(window, scale, true).as_str();
                    }
                    result += ")))"
                }
            }
            result += ")";
        }
        result += ")";

        write!(f, "{}", result.as_str())
    }
}
