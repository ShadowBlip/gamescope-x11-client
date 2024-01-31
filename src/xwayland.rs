use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};
use std::{borrow::Borrow, sync::mpsc::Sender};

use x11rb::{
    connection::Connection,
    protocol::xproto::{ChangeWindowAttributesAux, ConnectionExt, EventMask},
    rust_connection::RustConnection,
};

use crate::{
    atoms::GamescopeAtom,
    x11::{self, get_window_name},
};

/// Gamescope is hard-coded to look for STEAM_GAME=769 to determine if it is the
/// overlay app.
pub const OVERLAY_APP_ID: u32 = 769;

// Gamescope blur modes
pub enum BlurMode {
    Off,
    Cond,
    Always,
}

/// [XWayland] is a handle to a single Gamescope XWayland instance.
#[derive(Debug)]
pub struct XWayland {
    name: String,
    conn: Option<RustConnection>,
    root_window_id: u32,
}

impl XWayland {
    /// Create a new Gamescope XWayland instance with the given display name (e.g. ":0")
    pub fn new(name: String) -> Self {
        Self {
            name,
            conn: None,
            root_window_id: 0,
        }
    }
}

impl XWayland {
    /// Returns the name of the XWayland instance (E.g. ":0")
    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    /// Borrow the connection to the XWayland server. Will error if not yet
    /// connected.
    fn get_connection(&self) -> Result<&RustConnection, Box<dyn std::error::Error>> {
        if self.conn.is_none() {
            return Err("No connection".into());
        }

        let borrow = self.conn.borrow();
        let rust_connection = borrow.as_ref().unwrap();
        Ok(rust_connection)
    }

    /// Connect to the XWayland display
    pub fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Connect to the display
        let (conn, screen_num) = x11rb::connect(Some(self.name.as_str()))?;
        println!("Connected to: {}", screen_num);
        let screen = &conn.setup().roots[screen_num];

        self.root_window_id = screen.root;
        self.conn = Some(conn);

        Ok(())
    }

    /// Tries to discover the process IDs that are associated with the given
    /// window.
    pub fn get_pids_for_window(
        &self,
        window_id: u32,
    ) -> Result<Vec<u32>, Box<dyn std::error::Error>> {
        let conn = self.get_connection()?;
        x11::get_window_pids(conn, window_id)
    }

    /// Returns the window id(s) for the given process ID.
    pub fn get_windows_for_pid(&self, pid: u32) -> Result<Vec<u32>, Box<dyn std::error::Error>> {
        // Get all windows from the root window to search for the one with this
        // process ID.
        let all_windows = self.get_all_windows(self.root_window_id)?;
        let window_ids = all_windows
            .into_iter()
            .filter(|window_id| {
                let window_pid = self
                    .get_window_pid(*window_id)
                    .unwrap_or_default()
                    .unwrap_or_default();
                pid == window_pid
            })
            .collect();
        Ok(window_ids)
    }

    /// Listen for property changes on the root window
    pub fn listen_for_property_changes(
        &self,
    ) -> Result<(JoinHandle<()>, Receiver<String>), Box<dyn std::error::Error>> {
        self.listen_for_window_property_changes(self.root_window_id)
    }

    /// Listen for events and property changes on the given window. Returns a
    /// join handle of the listening thread and a receiver channel that can be
    /// used to receive property changes.
    /// https://stackoverflow.com/questions/60141048/get-notifications-when-active-x-window-changes-using-python-xlib
    pub fn listen_for_window_property_changes(
        &self,
        window_id: u32,
    ) -> Result<(JoinHandle<()>, Receiver<String>), Box<dyn std::error::Error>> {
        // Create a new connection for the new thread
        let (conn, _) = x11rb::connect(Some(self.name.as_str()))?;

        // Set the event mask to start listening for events
        let mut attrs = ChangeWindowAttributesAux::new();
        attrs.event_mask = Some(EventMask::PROPERTY_CHANGE);
        let result = conn.change_window_attributes(window_id, &attrs)?;
        result.check()?;

        // Create a channel to send update messages through
        let (tx, rx): (Sender<String>, Receiver<String>) = mpsc::channel();

        // Spawn a thread to listen for events
        let child = thread::spawn(move || {
            // Loop and listen for events
            loop {
                let event = conn.wait_for_event();
                if event.is_err() {
                    break;
                }
                let event = event.unwrap();

                // We only care about property change events
                let event = if let x11rb::protocol::Event::PropertyNotify(event) = event {
                    Some(event)
                } else {
                    None
                };
                if event.is_none() {
                    continue;
                }
                let event = event.unwrap();
                let atom = conn.get_atom_name(event.atom).unwrap().reply().unwrap();
                let property = String::from_utf8(atom.name).unwrap();

                tx.send(property).unwrap();
            }
        });

        Ok((child, rx))
    }

    /// Returns true if this instance is the primary Gamescope xwayland instance
    pub fn is_primary_instance(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let root_id = self.root_window_id;
        self.has_xprop(root_id, GamescopeAtom::KeyboardFocusDisplay)
    }

    /// Returns the root window ID of the xwayland instance
    pub fn get_root_window_id(&self) -> Result<u32, Box<dyn std::error::Error>> {
        let _ = self.get_connection()?;
        Ok(self.root_window_id)
    }

    /// Returns the window name of the given window
    pub fn get_window_name(
        &self,
        window_id: u32,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let conn = self.get_connection()?;
        x11::get_window_name(conn, window_id)
    }

    /// Returns the window ids of the children of the given window
    pub fn get_window_children(
        &self,
        window_id: u32,
    ) -> Result<Vec<u32>, Box<dyn std::error::Error>> {
        let conn = self.get_connection()?;
        let results = conn.query_tree(window_id)?.reply()?;
        Ok(results.children)
    }

    /// Recursively returns all child windows of the given window id
    pub fn get_all_windows(&self, window_id: u32) -> Result<Vec<u32>, Box<dyn std::error::Error>> {
        let children = self.get_window_children(window_id)?;
        if children.is_empty() {
            return Ok(Vec::new());
        }

        let mut leaves: Vec<u32> = Vec::new();
        for child in children {
            leaves.push(child);
            leaves.append(&mut self.get_all_windows(child)?);
        }

        Ok(leaves)
    }

    /// Returns the true if the given property exists on the given window
    pub fn has_xprop(
        &self,
        window_id: u32,
        key: GamescopeAtom,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let conn = self.get_connection()?;
        x11::has_property(conn, window_id, key.to_string().as_str())
    }

    /// Returns the value(s) of the given property on the given window
    pub fn get_xprop(
        &self,
        window_id: u32,
        key: GamescopeAtom,
    ) -> Result<Option<Vec<u32>>, Box<dyn std::error::Error>> {
        let conn = self.get_connection()?;
        x11::get_property(conn, window_id, key.to_string().as_str())
    }

    /// Returns the first value of the given property on the given window
    pub fn get_one_xprop(
        &self,
        window_id: u32,
        key: GamescopeAtom,
    ) -> Result<Option<u32>, Box<dyn std::error::Error>> {
        let value = self
            .get_xprop(window_id, key)?
            .unwrap_or_default()
            .drain(..)
            .next();
        Ok(value)
    }

    /// Sets the given x window property value(s) on the given window
    pub fn set_xprop(
        &self,
        window_id: u32,
        key: GamescopeAtom,
        values: Vec<u32>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.get_connection()?;
        x11::set_property(conn, window_id, key.to_string().as_str(), values)?;

        Ok(())
    }

    /// Removes the given x window property from the given window
    pub fn remove_xprop(
        &self,
        window_id: u32,
        key: GamescopeAtom,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.get_connection()?;
        x11::remove_property(conn, window_id, key.to_string().as_str())?;

        Ok(())
    }

    /// Returns the process ID of the given window from the '_NET_WM_PID' atom
    pub fn get_window_pid(
        &self,
        window_id: u32,
    ) -> Result<Option<u32>, Box<dyn std::error::Error>> {
        self.get_one_xprop(window_id, GamescopeAtom::NetWmPID)
    }

    /// Returns the currently set app ID on the given window
    pub fn get_app_id(&self, window_id: u32) -> Result<Option<u32>, Box<dyn std::error::Error>> {
        self.get_one_xprop(window_id, GamescopeAtom::SteamGame)
    }

    /// Sets the app ID on the given window
    pub fn set_app_id(
        &self,
        window_id: u32,
        app_id: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_xprop(window_id, GamescopeAtom::SteamGame, vec![app_id])
    }

    /// Returns whether or not the given window has an app ID set
    pub fn has_app_id(&self, window_id: u32) -> Result<bool, Box<dyn std::error::Error>> {
        self.has_xprop(window_id, GamescopeAtom::SteamGame)
    }
}

/// A Primary [XWayland] has extra window properties available for controlling
/// Gamescope.
pub trait Primary {
    /// Return a list of focusable apps
    fn get_focusable_apps(&self) -> Result<Option<Vec<u32>>, Box<dyn std::error::Error>>;
    /// Returns true if the window with the given window ID exists in focusable apps
    fn is_focusable_app(&self, window_id: u32) -> Result<bool, Box<dyn std::error::Error>>;
    /// Returns a list of focusable window ids
    fn get_focusable_windows(&self) -> Result<Option<Vec<u32>>, Box<dyn std::error::Error>>;
    /// Returns a list of focusable window names
    fn get_focusable_window_names(&self) -> Result<Vec<String>, Box<dyn std::error::Error>>;
    /// Return the currently focused window id.
    fn get_focused_window(&self) -> Result<Option<u32>, Box<dyn std::error::Error>>;
    /// Return the currently focused app id.
    fn get_focused_app(&self) -> Result<Option<u32>, Box<dyn std::error::Error>>;
    /// Return the currently focused gfx app id.
    fn get_focused_app_gfx(&self) -> Result<Option<u32>, Box<dyn std::error::Error>>;
    /// Sets the given window as the main launcher app.
    fn set_main_app(&self, window_id: u32) -> Result<(), Box<dyn std::error::Error>>;
    /// Set the given window as the primary overlay input focus. This should be set to
    /// "1" whenever the overlay wants to intercept input from a game.
    fn set_input_focus(&self, window_id: u32, value: u32)
        -> Result<(), Box<dyn std::error::Error>>;
    /// Returns whether or not the overlay window is currently focused
    fn is_overlay_focused(&self) -> Result<bool, Box<dyn std::error::Error>>;
    /// Get the overlay status for the given window
    fn get_overlay(&self, window_id: u32) -> Result<Option<u32>, Box<dyn std::error::Error>>;
    /// Set the given window as the overlay window
    fn set_overlay(&self, window_id: u32, value: u32) -> Result<(), Box<dyn std::error::Error>>;
    /// Set the given window as a notification. This should be set to "1" when some
    /// UI wants to be shown but not intercept input.
    fn set_notification(
        &self,
        window_id: u32,
        value: u32,
    ) -> Result<(), Box<dyn std::error::Error>>;
    /// Set the given window as an external overlay
    fn set_external_overlay(
        &self,
        window_id: u32,
        value: u32,
    ) -> Result<(), Box<dyn std::error::Error>>;
    /// Sets the Gamescope FPS limit
    fn set_fps_limit(&self, fps: u32) -> Result<(), Box<dyn std::error::Error>>;
    /// Gets the current Gamescope FPS limit
    fn get_fps_limit(&self) -> Result<Option<u32>, Box<dyn std::error::Error>>;
    /// Sets the Gamescope blur mode
    fn set_blur_mode(&self, mode: BlurMode) -> Result<(), Box<dyn std::error::Error>>;
    /// Gets the Gamescope blur mode
    fn get_blur_mode(&self) -> Result<Option<BlurMode>, Box<dyn std::error::Error>>;
    /// Sets the Gamescope blur radius when blur is active
    fn set_blur_radius(&self, radius: u32) -> Result<(), Box<dyn std::error::Error>>;
    /// Configures Gamescope to allow tearing or not
    fn set_allow_tearing(&self, allow: bool) -> Result<(), Box<dyn std::error::Error>>;
    /// Returns the currently set manual focus
    fn get_baselayer_window(&self) -> Result<Option<u32>, Box<dyn std::error::Error>>;
    /// Focuses the given window
    fn set_baselayer_window(&self, window_id: u32) -> Result<(), Box<dyn std::error::Error>>;
    /// Removes the baselayer property to un-focus windows
    fn remove_baselayer_window(&self) -> Result<(), Box<dyn std::error::Error>>;
    /// Request a screenshot from Gamescope
    fn request_screenshot(&self) -> Result<(), Box<dyn std::error::Error>>;
}

impl Primary for XWayland {
    fn get_focusable_apps(&self) -> Result<Option<Vec<u32>>, Box<dyn std::error::Error>> {
        self.get_xprop(self.root_window_id, GamescopeAtom::FocusableApps)
    }

    fn is_focusable_app(&self, window_id: u32) -> Result<bool, Box<dyn std::error::Error>> {
        let focusable = self.get_focusable_apps()?;
        if let Some(focusable) = focusable {
            Ok(focusable.contains(&window_id))
        } else {
            Ok(false)
        }
    }

    fn get_focusable_windows(&self) -> Result<Option<Vec<u32>>, Box<dyn std::error::Error>> {
        self.get_xprop(self.root_window_id, GamescopeAtom::FocusableWindows)
    }

    fn get_focusable_window_names(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let conn = self.get_connection()?;
        let focusable_windows = self.get_focusable_windows()?.unwrap_or_default();
        let mut window_names: Vec<String> = Vec::new();
        for window in focusable_windows {
            let window_name = get_window_name(conn, window)?;
            if let Some(window_name) = window_name {
                window_names.push(window_name);
            }
        }

        Ok(window_names)
    }

    fn get_focused_window(&self) -> Result<Option<u32>, Box<dyn std::error::Error>> {
        self.get_one_xprop(self.root_window_id, GamescopeAtom::FocusedWindow)
    }

    fn get_focused_app(&self) -> Result<Option<u32>, Box<dyn std::error::Error>> {
        self.get_one_xprop(self.root_window_id, GamescopeAtom::FocusedApp)
    }

    fn get_focused_app_gfx(&self) -> Result<Option<u32>, Box<dyn std::error::Error>> {
        self.get_one_xprop(self.root_window_id, GamescopeAtom::FocusedAppGFX)
    }

    fn set_main_app(&self, window_id: u32) -> Result<(), Box<dyn std::error::Error>> {
        self.set_xprop(window_id, GamescopeAtom::SteamGame, vec![OVERLAY_APP_ID])
    }

    fn set_input_focus(
        &self,
        window_id: u32,
        value: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_xprop(window_id, GamescopeAtom::SteamInputFocus, vec![value])
    }

    fn is_overlay_focused(&self) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(self.get_focused_app()?.unwrap_or_default() == OVERLAY_APP_ID)
    }

    fn get_overlay(&self, window_id: u32) -> Result<Option<u32>, Box<dyn std::error::Error>> {
        self.get_one_xprop(window_id, GamescopeAtom::SteamOverlay)
    }

    fn set_overlay(&self, window_id: u32, value: u32) -> Result<(), Box<dyn std::error::Error>> {
        self.set_xprop(window_id, GamescopeAtom::SteamOverlay, vec![value])
    }

    fn set_notification(
        &self,
        window_id: u32,
        value: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_xprop(window_id, GamescopeAtom::SteamNotification, vec![value])
    }

    fn set_external_overlay(
        &self,
        window_id: u32,
        value: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_xprop(window_id, GamescopeAtom::ExternalOverlay, vec![value])
    }

    fn set_fps_limit(&self, fps: u32) -> Result<(), Box<dyn std::error::Error>> {
        self.set_xprop(self.root_window_id, GamescopeAtom::FPSLimit, vec![fps])
    }

    fn get_fps_limit(&self) -> Result<Option<u32>, Box<dyn std::error::Error>> {
        self.get_one_xprop(self.root_window_id, GamescopeAtom::FPSLimit)
    }

    fn set_blur_mode(&self, mode: BlurMode) -> Result<(), Box<dyn std::error::Error>> {
        let mode = match mode {
            BlurMode::Off => 0,
            BlurMode::Cond => 1,
            BlurMode::Always => 2,
        };
        self.set_xprop(self.root_window_id, GamescopeAtom::FPSLimit, vec![mode])
    }

    fn get_blur_mode(&self) -> Result<Option<BlurMode>, Box<dyn std::error::Error>> {
        let mode = self.get_one_xprop(self.root_window_id, GamescopeAtom::BlurMode)?;
        if mode.is_none() {
            return Ok(None);
        }

        match mode.unwrap() {
            0 => Ok(Some(BlurMode::Off)),
            1 => Ok(Some(BlurMode::Cond)),
            2 => Ok(Some(BlurMode::Always)),
            _ => Ok(None),
        }
    }

    fn set_blur_radius(&self, radius: u32) -> Result<(), Box<dyn std::error::Error>> {
        self.set_xprop(self.root_window_id, GamescopeAtom::BlurRadius, vec![radius])
    }

    fn set_allow_tearing(&self, allow: bool) -> Result<(), Box<dyn std::error::Error>> {
        let value = if allow { 1 } else { 0 };
        self.set_xprop(
            self.root_window_id,
            GamescopeAtom::AllowTearing,
            vec![value],
        )
    }

    fn get_baselayer_window(&self) -> Result<Option<u32>, Box<dyn std::error::Error>> {
        self.get_one_xprop(self.root_window_id, GamescopeAtom::BaselayerWindow)
    }

    fn set_baselayer_window(&self, window_id: u32) -> Result<(), Box<dyn std::error::Error>> {
        self.set_xprop(
            self.root_window_id,
            GamescopeAtom::BaselayerWindow,
            vec![window_id],
        )
    }

    fn remove_baselayer_window(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.remove_xprop(self.root_window_id, GamescopeAtom::BaselayerWindow)
    }

    fn request_screenshot(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.set_xprop(
            self.root_window_id,
            GamescopeAtom::RequestScreenshot,
            vec![1],
        )
    }
}
