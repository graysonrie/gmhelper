use anyhow::{Result, bail};
use enigo::{Button, Coordinate, Direction, Enigo, Mouse, Settings};
use std::{thread, time::Duration};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, RECT},
        UI::WindowsAndMessaging::{
            EnumWindows, GetWindowRect, GetWindowTextLengthW, GetWindowTextW, IsWindowVisible,
            SW_RESTORE, SetForegroundWindow, ShowWindow,
        },
    },
    core::BOOL,
};

/// Focused the gamemaker window, goes to the Asset Browser, and clicks Expand All
pub fn focus_gamemaker_window(try_open_asset_browser: bool) -> Result<()> {
    thread::sleep(Duration::from_millis(1000));

    let hwnd = find_window_containing("GameMaker")?;

    unsafe {
        let _ = ShowWindow(hwnd, SW_RESTORE);
        if !SetForegroundWindow(hwnd).as_bool() {
            bail!("SetForegroundWindow failed");
        }
    }

    if !try_open_asset_browser {
        return Ok(());
    }

    thread::sleep(Duration::from_millis(800));

    let rect = get_window_rect(hwnd)?;

    // Adjust these once, then put them in config.
    let asset_browser_folder_x = rect.right - 180;
    let asset_browser_folder_y = rect.top + 260;

    let expand_all_x = asset_browser_folder_x - 65;
    let expand_all_y = asset_browser_folder_y + 200;

    let mut enigo = Enigo::new(&Settings::default())?;

    enigo.move_mouse(
        asset_browser_folder_x,
        asset_browser_folder_y,
        Coordinate::Abs,
    )?;
    enigo.button(Button::Right, Direction::Click)?;

    thread::sleep(Duration::from_millis(10));

    enigo.move_mouse(expand_all_x, expand_all_y, Coordinate::Abs)?;
    enigo.button(Button::Left, Direction::Click)?;

    Ok(())
}

fn find_window_containing(needle: &str) -> Result<HWND> {
    struct SearchData {
        needle: String,
        result: Option<HWND>,
    }

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        unsafe {
            let data = &mut *(lparam.0 as *mut SearchData);

            if !IsWindowVisible(hwnd).as_bool() {
                return BOOL(1);
            }

            let len = GetWindowTextLengthW(hwnd);
            if len == 0 {
                return BOOL(1);
            }

            let mut buffer = vec![0u16; len as usize + 1];
            let copied = GetWindowTextW(hwnd, &mut buffer);

            if copied > 0 {
                let title = String::from_utf16_lossy(&buffer[..copied as usize]);

                if title.contains(&data.needle) {
                    data.result = Some(hwnd);
                    return BOOL(0);
                }
            }

            BOOL(1)
        }
    }

    let mut data = SearchData {
        needle: needle.to_string(),
        result: None,
    };

    // Stopping the callback with FALSE makes EnumWindows return FALSE too; `?` treats that as failure
    // (GetLastError is often still SUCCESS → "The operation completed successfully").
    unsafe {
        let _ = EnumWindows(
            Some(enum_proc),
            LPARAM(&mut data as *mut SearchData as isize),
        );
    }

    data.result
        .ok_or_else(|| anyhow::anyhow!("Could not find a visible window containing {needle:?}"))
}

fn get_window_rect(hwnd: HWND) -> Result<RECT> {
    unsafe {
        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect)?;
        Ok(rect)
    }
}
