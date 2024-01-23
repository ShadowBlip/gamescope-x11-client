use std::fs;

use x11rb::connection::Connection;

mod atoms;
mod x11;
mod xwayland;

// Returns instances to all available Gamescope XWaylands
pub fn discover_gamescope_xwaylands() -> Result<Vec<xwayland::XWayland>, Box<dyn std::error::Error>>
{
    let gamescope_displays = discover_gamescope_displays()?;
    let xwaylands = gamescope_displays
        .iter()
        .map(|display_name| xwayland::XWayland::new(display_name.into()))
        .collect();

    Ok(xwaylands)
}

/// Returns all gamescope xwayland names (E.g. [":0", ":1"])
pub fn discover_gamescope_displays() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Discover all x11 displays
    let x11_displays = discover_x11_displays()?;

    // Array of gamescope xwayland displays
    let mut gamescope_displays: Vec<String> = Vec::new();

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

        // Add the display name to the list of gamescope displays
        if x11::is_gamescope_xwayland(conn, root_window_id)? {
            println!("Found Gamescope xwayland: {}", display);
            gamescope_displays.push(display);
        }
    }

    Ok(gamescope_displays)
}

/// Returns all x11 display names (E.g. [":0", ":1"])
pub fn discover_x11_displays() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    println!("Discovering gamescope displays!");

    // Array of X11 displays
    let mut display_names: Vec<String> = Vec::new();

    // X11 displays have a corresponding socket in /tmp/.X11-unix
    // The sockets are named like: X0, X1, X2, etc.
    let sockets = fs::read_dir("/tmp/.X11-unix")?;

    // Loop through each socket file and derive the display number
    for socket in sockets {
        let dir_entry = &socket?;
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

    Ok(display_names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_gamescope_displays() {
        let xwaylands = discover_gamescope_xwaylands().unwrap();
        for mut xwayland in xwaylands {
            xwayland.connect().unwrap();
            //xwayland.get_focusable_apps();
            let is_primary = xwayland.is_primary_instance().unwrap();
            println!(
                "Found XWayland: {:?} {}",
                xwayland.get_name(),
                if is_primary { "(primary)" } else { "" }
            );
        }
    }
}
