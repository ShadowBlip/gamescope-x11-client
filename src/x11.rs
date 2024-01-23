use x11rb::{
    connection::Connection,
    protocol::{
        res::{ClientIdMask, ClientIdSpec},
        xproto::{intern_atom, AtomEnum, ConnectionExt, InputFocus, PropMode},
    },
    CURRENT_TIME,
};

use crate::atoms::GamescopeAtom;

/// Returns true if the given X server connection is a gamescope xwayland
pub fn is_gamescope_xwayland<F>(
    conn: F,
    root_window_id: u32,
) -> Result<bool, Box<dyn std::error::Error>>
where
    F: Connection,
{
    has_property(
        conn,
        root_window_id,
        GamescopeAtom::CursorVisibleFeedback.to_string().as_str(),
    )
}

pub fn get_string_property<F>(
    conn: F,
    window_id: u32,
    key: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>>
where
    F: Connection,
{
    let atom = intern_atom(&conn, false, key.as_bytes())?;
    let atom = atom.reply()?;

    // Request the property from the X server
    let response = conn.get_property(false, window_id, atom.atom, AtomEnum::STRING, 0, 8);
    let value = response?.reply()?;

    // Check to see if there was a value returned
    if value.value_len == 0 {
        return Ok(None);
    }

    let values: Vec<u8> = value.value8().unwrap().collect();
    Ok(Some(String::from_utf8(values)?))
}

/// Returns true if the given window has the given property
pub fn has_property<F>(
    conn: F,
    window_id: u32,
    key: &str,
) -> Result<bool, Box<dyn std::error::Error>>
where
    F: Connection,
{
    Ok(get_property(conn, window_id, key)?.is_some())
}

/// Returns the value of the given x property on the given window.
/// TODO: We assume everything is a cardinal
pub fn get_property<F>(
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

/// Sets the value(s) of the given x property on the given window.
pub fn set_property<F>(
    conn: F,
    window_id: u32,
    key: &str,
    values: Vec<u32>,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: Connection,
{
    change_property(conn, window_id, key, values, PropMode::REPLACE)
}

/// Append the value(s) of the given x property on the given window.
pub fn append_property<F>(
    conn: F,
    window_id: u32,
    key: &str,
    values: Vec<u32>,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: Connection,
{
    change_property(conn, window_id, key, values, PropMode::APPEND)
}

/// Prepend the value(s) of the given x property on the given window.
pub fn prepend_property<F>(
    conn: F,
    window_id: u32,
    key: &str,
    values: Vec<u32>,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: Connection,
{
    change_property(conn, window_id, key, values, PropMode::PREPEND)
}

/// Change the value(s) of the given x property on the given window.
pub fn change_property<F>(
    conn: F,
    window_id: u32,
    key: &str,
    values: Vec<u32>,
    mode: PropMode,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: Connection,
{
    use x11rb::wrapper::ConnectionExt;

    let atom = intern_atom(&conn, false, key.as_bytes())?;
    let atom = atom.reply()?;

    // Request setting the property
    let result = conn.change_property32(
        mode,
        window_id,
        atom.atom,
        AtomEnum::CARDINAL,
        values.as_slice(),
    )?;
    result.check()?;

    Ok(())
}

/// Remove the given x property from the given window.
pub fn remove_property<F>(
    conn: F,
    window_id: u32,
    key: &str,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: Connection,
{
    let atom = intern_atom(&conn, false, key.as_bytes())?;
    let atom = atom.reply()?;

    let result = conn.delete_property(window_id, atom.atom)?;
    result.check()?;

    Ok(())
}

/// Returns a list of all available properties on the given window
pub fn list_properties<F>(
    conn: F,
    window_id: u32,
) -> Result<Vec<String>, Box<dyn std::error::Error>>
where
    F: Connection,
{
    let results = conn.list_properties(window_id)?.reply()?;
    let mut properties: Vec<String> = Vec::new();

    for atom in results.atoms {
        let name = conn.get_atom_name(atom)?.reply()?.name;
        let name = String::from_utf8(name)?;
        properties.push(name);
    }

    Ok(properties)
}

/// Uses XRes to determine the given Window's PID
pub fn get_window_pids<F>(conn: F, window_id: u32) -> Result<Vec<u32>, Box<dyn std::error::Error>>
where
    F: Connection,
{
    use x11rb::protocol::res::ConnectionExt;
    let spec = ClientIdSpec {
        client: window_id,
        mask: ClientIdMask::LOCAL_CLIENT_PID,
    };

    let client_ids = conn.res_query_client_ids(&[spec])?.reply()?;

    let mut pids: Vec<u32> = Vec::new();
    for client_id in client_ids.ids {
        let mut client_pids = client_id.value;
        pids.append(&mut client_pids);
    }

    Ok(pids)
}

// Set input focus on the given window
pub fn set_input_focus<F>(conn: F, window_id: u32) -> Result<(), Box<dyn std::error::Error>>
where
    F: Connection,
{
    conn.set_input_focus(InputFocus::NONE, window_id, CURRENT_TIME)?;

    Ok(())
}

// Returns the window name of the given window
pub fn get_window_name<F>(
    conn: F,
    window_id: u32,
) -> Result<Option<String>, Box<dyn std::error::Error>>
where
    F: Connection,
{
    get_string_property(conn, window_id, "WM_NAME")
}
