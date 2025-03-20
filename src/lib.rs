use std::{
    fs,
    io::{self, BufRead},
};

use x11rb::connection::Connection;

pub mod atoms;
mod x11;
pub mod xwayland;

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
        // Connect to the display
        let result = x11rb::connect(Some(display.as_str()));
        if result.is_err() {
            continue;
        }
        let (conn, screen_num) = result.unwrap();
        let screen = &conn.setup().roots[screen_num];

        let root_window_id = screen.root;

        // Add the display name to the list of gamescope displays
        if x11::is_gamescope_xwayland(conn, root_window_id)? {
            gamescope_displays.push(display);
        }
    }

    Ok(gamescope_displays)
}

/// Returns all x11 display names (E.g. [":0", ":1"])
pub fn discover_x11_displays() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Array of X11 displays
    let mut display_names: Vec<String> = Vec::new();

    // Find the all the sockets tracked by the kernel
    let socks = fs::File::open("/proc/net/unix")?;
    let prefix = "/tmp/.X11-unix/X";
    for line in io::BufReader::new(socks).lines() {
        let Ok(line) = line else {
            continue;
        };

        // Get the 5th field, which is the socket path
        let Some(sock) = line.split_whitespace().last() else {
            continue;
        };

        if !sock.starts_with(prefix) {
            continue;
        }

        // Get the path's suffix, which is our display name
        let Some(suffix) = sock.strip_prefix(prefix) else {
            continue;
        };

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

            //xwayland.listen_for_property_changes().unwrap();
        }
    }
}
