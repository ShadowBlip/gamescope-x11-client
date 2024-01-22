use std::fs;

use x11rb::{
    connection::Connection,
    protocol::xproto::{intern_atom, Atom, AtomEnum, ConnectionExt, InternAtomRequest},
};

mod atoms;

fn is_gamescope_xwayland() {}

/// Returns true if the given window has the given property
fn has_property<F>(conn: F, window_id: u32, key: &str) -> Result<bool, Box<dyn std::error::Error>>
where
    F: Connection,
{
    Ok(get_property(conn, window_id, key)?.is_some())
}

/// Returns the value of the given x property on the given window.
/// TODO: We assume everything is a cardinal
fn get_property<F>(
    conn: F,
    window_id: u32,
    key: &str,
) -> Result<Option<Vec<u32>>, Box<dyn std::error::Error>>
where
    F: Connection,
{
    let atom = intern_atom(&conn, false, key.as_bytes())?;
    let atom = atom.reply()?;

    // Request the property from the X server
    let response = conn.get_property(
        false,
        window_id,
        atom.atom,
        AtomEnum::CARDINAL,
        0,
        u32::max_value(),
    );
    let value = response?.reply()?;

    // Check to see if there was a value returned
    if value.value_len == 0 {
        return Ok(None);
    }

    let values: Vec<u32> = value.value32().unwrap().collect();
    Ok(Some(values))
}

/// Returns all gamescope xwayland names (E.g. [":0", ":1"])
pub fn discover_gamescope_displays() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Discover all x11 displays
    let x11_displays = discover_x11_displays();

    // Array of gamescope xwayland displays
    let gamescope_displays: Vec<String> = Vec::new();

    // Check to see if the root window of these displays has gamescope-specific properties
    for display in x11_displays {
        println!("Trying to connect to display: {}", display);
        // Connect to the display
        let result = x11rb::connect(Some(display.as_str()));
        if result.is_err() {
            println!("Failed to connect to display: {}", display);
            continue;
        }
        let (conn, screen_num) = result.unwrap();
        println!("Connected to: {}", screen_num);
        let screen = &conn.setup().roots[screen_num];

        let root_window_id = screen.root;

        //get_property(conn, root_window_id, "GAMESCOPE_FOCUSABLE_WINDOWS");
        get_property(conn, root_window_id, atoms::FOCUSABLE_WINDOWS);
    }

    Ok(gamescope_displays)
}

/// Returns all x11 display names (E.g. [":0", ":1"])
pub fn discover_x11_displays() -> Vec<String> {
    println!("Discovering gamescope displays!");

    // Array of X11 displays
    let mut display_names: Vec<String> = Vec::new();

    // X11 displays have a corresponding socket in /tmp/.X11-unix
    // The sockets are named like: X0, X1, X2, etc.
    let sockets = fs::read_dir("/tmp/.X11-unix").unwrap();

    // Loop through each socket file and derive the display number
    for socket in sockets {
        let dir_entry = &socket.unwrap();
        let path = &dir_entry.path();

        // Get the name of the socket (E.g. "X0") and get its suffix
        // (E.g. "0")
        let socket = path.file_name().unwrap().to_os_string();
        let socket = socket.into_string();
        let socket = &socket.unwrap();
        let suffix = socket.strip_prefix('X').unwrap();

        // Skip X11 sockets with weird names
        if suffix.parse::<u64>().is_err() {
            continue;
        }

        display_names.push(format!(":{}", suffix));
    }

    display_names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_gamescope_displayed() {
        discover_gamescope_displays();
    }
}
