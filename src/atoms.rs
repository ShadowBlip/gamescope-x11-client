#[derive(Debug, strum_macros::Display)]
pub enum GamescopeAtom {
    #[strum(serialize = "_NET_WM_PID")]
    NetWmPID,
    #[strum(serialize = "STEAM_BIGPICTURE")]
    Steam,
    #[strum(serialize = "GAMESCOPE_INPUT_COUNTER")]
    InputCounter,
    #[strum(serialize = "GAMESCOPE_FOCUSED_APP")]
    FocusedApp,
    #[strum(serialize = "GAMESCOPE_FOCUSED_APP_GFX")]
    FocusedAppGFX,
    #[strum(serialize = "GAMESCOPE_FOCUSED_WINDOW")]
    FocusedWindow,
    #[strum(serialize = "GAMESCOPE_FOCUSABLE_APPS")]
    FocusableApps,
    #[strum(serialize = "GAMESCOPE_FOCUSABLE_WINDOWS")]
    FocusableWindows,
    #[strum(serialize = "GAMESCOPE_CURSOR_VISIBLE_FEEDBACK")]
    CursorVisibleFeedback,
    #[strum(serialize = "GAMESCOPE_EXTERNAL_OVERLAY")]
    ExternalOverlay,
    #[strum(serialize = "GAMESCOPE_FPS_LIMIT")]
    FPSLimit,
    #[strum(serialize = "GAMESCOPE_BLUR_MODE")]
    BlurMode,
    #[strum(serialize = "GAMESCOPE_BLUR_RADIUS")]
    BlurRadius,
    #[strum(serialize = "GAMESCOPE_ALLOW_TEARING")]
    AllowTearing,
    #[strum(serialize = "GAMESCOPECTRL_BASELAYER_WINDOW")]
    BaselayerWindow,
    #[strum(serialize = "GAMESCOPECTRL_REQUEST_SCREENSHOT")]
    RequestScreenshot,
    #[strum(serialize = "STEAM_GAME")]
    SteamGame,
    #[strum(serialize = "STEAM_INPUT_FOCUS")]
    SteamInputFocus,
    #[strum(serialize = "STEAM_OVERLAY")]
    SteamOverlay,
    #[strum(serialize = "STEAM_NOTIFICATION")]
    SteamNotification,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atom_enums() {
        assert_eq!(
            "GAMESCOPE_FOCUSABLE_WINDOWS",
            GamescopeAtom::FocusableWindows.to_string()
        );
    }
}
