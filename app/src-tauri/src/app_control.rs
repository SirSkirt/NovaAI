use serde::Serialize;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppWindowInfo {
    pub window_id: String,
    pub title: String,
    pub process_name: String,
    pub pid: u32,
    pub chatgpt_eligible: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppControlGrant {
    pub grant_id: String,
    pub adapter: String,
    pub window_id: String,
    pub title: String,
    pub process_name: String,
    pub pid: u32,
    pub capabilities: Vec<String>,
}

#[derive(Default)]
pub struct AppControlState {
    grant: Mutex<Option<AppControlGrant>>,
}

fn new_grant_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("grant-{nanos}")
}

#[cfg(target_os = "windows")]
mod platform {
    use super::{AppControlGrant, AppControlState, AppWindowInfo, new_grant_id};
    use std::{path::Path, thread, time::Duration};
    use tauri::State;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HWND, LPARAM},
        System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION},
        UI::{
            Input::KeyboardAndMouse::{keybd_event, KEYEVENTF_KEYUP, VK_CONTROL, VK_RETURN},
            WindowsAndMessaging::{
                EnumWindows, GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
                GetWindowThreadProcessId, IsWindow, IsWindowVisible, SetForegroundWindow,
                ShowWindow, SW_RESTORE,
            },
        },
    };

    const MAX_CAPTURE_CHARS: usize = 20_000;
    const MAX_SEND_CHARS: usize = 12_000;

    fn browser_process(name: &str) -> bool {
        matches!(
            name.to_ascii_lowercase().as_str(),
            "firefox.exe" | "chrome.exe" | "msedge.exe" | "brave.exe" | "vivaldi.exe" | "arc.exe"
        )
    }

    unsafe fn window_title(hwnd: HWND) -> String {
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 { return String::new(); }
        let mut buffer = vec![0u16; len as usize + 1];
        let written = GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32);
        if written <= 0 { return String::new(); }
        String::from_utf16_lossy(&buffer[..written as usize])
    }

    unsafe fn process_name(pid: u32) -> String {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() { return String::new(); }
        let mut buffer = vec![0u16; 32768];
        let mut size = buffer.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut size);
        let _ = CloseHandle(handle);
        if ok == 0 || size == 0 { return String::new(); }
        let full = String::from_utf16_lossy(&buffer[..size as usize]);
        Path::new(&full)
            .file_name()
            .map(|x| x.to_string_lossy().to_string())
            .unwrap_or(full)
    }

    unsafe fn snapshot(hwnd: HWND) -> Option<AppWindowInfo> {
        if IsWindow(hwnd) == 0 || IsWindowVisible(hwnd) == 0 { return None; }
        let title = window_title(hwnd);
        if title.trim().is_empty() { return None; }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 || pid == std::process::id() { return None; }
        let process_name = process_name(pid);
        let chatgpt_eligible = browser_process(&process_name)
            && title.to_ascii_lowercase().contains("chatgpt");
        Some(AppWindowInfo {
            window_id: format!("{}", hwnd as usize),
            title,
            process_name,
            pid,
            chatgpt_eligible,
        })
    }

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> i32 {
        let list = &mut *(lparam as *mut Vec<AppWindowInfo>);
        if let Some(info) = snapshot(hwnd) { list.push(info); }
        1
    }

    fn list_windows_inner() -> Vec<AppWindowInfo> {
        let mut list: Vec<AppWindowInfo> = Vec::new();
        unsafe {
            EnumWindows(Some(enum_proc), (&mut list as *mut Vec<AppWindowInfo>) as LPARAM);
        }
        list.sort_by(|a,b| a.process_name.cmp(&b.process_name).then(a.title.cmp(&b.title)));
        list
    }

    fn parse_window_id(id: &str) -> Result<HWND, String> {
        let raw = id.parse::<usize>().map_err(|_| "Invalid window identifier.".to_string())?;
        if raw == 0 { return Err("Invalid window identifier.".into()); }
        Ok(raw as HWND)
    }

    fn current_snapshot(id: &str) -> Result<AppWindowInfo, String> {
        let hwnd = parse_window_id(id)?;
        unsafe { snapshot(hwnd) }.ok_or_else(|| "The approved window is no longer available.".to_string())
    }

    fn validate_grant(grant: &AppControlGrant) -> Result<(HWND, AppWindowInfo), String> {
        let hwnd = parse_window_id(&grant.window_id)?;
        let now = current_snapshot(&grant.window_id)?;
        if now.pid != grant.pid || !now.process_name.eq_ignore_ascii_case(&grant.process_name) {
            return Err("The approved window no longer belongs to the approved process. Access was refused.".into());
        }
        if grant.adapter == "chatgpt_browser" {
            if !now.chatgpt_eligible {
                return Err("ChatGPT mode requires the approved browser window to remain on a window whose title contains ChatGPT.".into());
            }
        }
        Ok((hwnd, now))
    }

    fn approved(state: &State<'_, AppControlState>) -> Result<AppControlGrant, String> {
        state.grant.lock().map_err(|_| "App-control state is unavailable.".to_string())?
            .clone().ok_or_else(|| "No application is approved for this session.".to_string())
    }

    fn focus_grant(grant: &AppControlGrant) -> Result<(), String> {
        let (hwnd, _) = validate_grant(grant)?;
        unsafe {
            ShowWindow(hwnd, SW_RESTORE);
            if SetForegroundWindow(hwnd) == 0 {
                return Err("Windows refused to focus the approved application.".into());
            }
        }
        thread::sleep(Duration::from_millis(90));
        let foreground = unsafe { GetForegroundWindow() };
        if foreground != hwnd {
            return Err("The approved application did not become the foreground window, so Nova refused to send input.".into());
        }
        Ok(())
    }

    unsafe fn key_down(vk: u8) { keybd_event(vk, 0, 0, 0); }
    unsafe fn key_up(vk: u8) { keybd_event(vk, 0, KEYEVENTF_KEYUP, 0); }

    fn ctrl_key(vk: u8) {
        unsafe {
            key_down(VK_CONTROL as u8);
            key_down(vk);
            key_up(vk);
            key_up(VK_CONTROL as u8);
        }
    }

    fn press_enter() {
        unsafe {
            key_down(VK_RETURN as u8);
            key_up(VK_RETURN as u8);
        }
    }

    fn paste_text_to_grant(grant: &AppControlGrant, text: &str, enter: bool) -> Result<(), String> {
        if text.chars().count() > MAX_SEND_CHARS {
            return Err(format!("Refusing to send more than {MAX_SEND_CHARS} characters in one action."));
        }
        let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("Clipboard unavailable: {e}"))?;
        let previous = clipboard.get_text().ok();
        clipboard.set_text(text.to_string()).map_err(|e| format!("Could not stage text for the approved app: {e}"))?;
        drop(clipboard);

        focus_grant(grant)?;
        ctrl_key(b'V');
        thread::sleep(Duration::from_millis(120));
        if enter {
            press_enter();
            thread::sleep(Duration::from_millis(60));
        }

        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if let Some(old) = previous {
                let _ = clipboard.set_text(old);
            } else {
                let _ = clipboard.set_text(String::new());
            }
        }
        Ok(())
    }

    pub fn list_windows() -> Result<Vec<AppWindowInfo>, String> { Ok(list_windows_inner()) }

    pub fn begin(window_id: String, adapter: String, state: State<'_, AppControlState>) -> Result<AppControlGrant, String> {
        let selected = current_snapshot(&window_id)?;
        let adapter = match adapter.as_str() {
            "chatgpt_browser" => {
                if !selected.chatgpt_eligible {
                    return Err("ChatGPT mode only accepts a visible browser window whose title contains ChatGPT.".into());
                }
                adapter
            }
            "generic_text" => adapter,
            _ => return Err("Unsupported app-control adapter.".into()),
        };

        let grant = AppControlGrant {
            grant_id: new_grant_id(),
            adapter,
            window_id: selected.window_id.clone(),
            title: selected.title.clone(),
            process_name: selected.process_name.clone(),
            pid: selected.pid,
            capabilities: vec![
                "focus-approved-window".into(),
                "type-text-into-approved-window".into(),
                "press-enter-in-approved-window".into(),
                "capture-explicit-selection-from-approved-window".into(),
            ],
        };
        *state.grant.lock().map_err(|_| "App-control state is unavailable.".to_string())? = Some(grant.clone());
        Ok(grant)
    }

    pub fn end(state: State<'_, AppControlState>) -> Result<(), String> {
        *state.grant.lock().map_err(|_| "App-control state is unavailable.".to_string())? = None;
        Ok(())
    }

    pub fn status(state: State<'_, AppControlState>) -> Result<Option<AppControlGrant>, String> {
        Ok(state.grant.lock().map_err(|_| "App-control state is unavailable.".to_string())?.clone())
    }

    pub fn focus(state: State<'_, AppControlState>) -> Result<(), String> {
        let grant = approved(&state)?;
        focus_grant(&grant)
    }

    pub fn type_text(text: String, state: State<'_, AppControlState>) -> Result<(), String> {
        let grant = approved(&state)?;
        paste_text_to_grant(&grant, &text, false)
    }

    pub fn type_and_enter(text: String, state: State<'_, AppControlState>) -> Result<(), String> {
        let grant = approved(&state)?;
        paste_text_to_grant(&grant, &text, true)
    }

    pub fn enter(state: State<'_, AppControlState>) -> Result<(), String> {
        let grant = approved(&state)?;
        focus_grant(&grant)?;
        press_enter();
        Ok(())
    }

    pub fn capture_selection(state: State<'_, AppControlState>) -> Result<String, String> {
        let grant = approved(&state)?;
        let marker = format!("__NOVA_SELECTION_MARKER_{}__", new_grant_id());
        let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("Clipboard unavailable: {e}"))?;
        let previous = clipboard.get_text().ok();
        clipboard.set_text(marker.clone()).map_err(|e| format!("Could not prepare selection capture: {e}"))?;
        drop(clipboard);

        focus_grant(&grant)?;
        ctrl_key(b'C');
        thread::sleep(Duration::from_millis(180));

        let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("Clipboard unavailable after copy: {e}"))?;
        let captured = clipboard.get_text().map_err(|e| format!("The approved application did not provide text: {e}"))?;
        if let Some(old) = previous { let _ = clipboard.set_text(old); } else { let _ = clipboard.set_text(String::new()); }

        if captured == marker || captured.trim().is_empty() {
            return Err("No selected text was copied. Select the text inside the approved application and try Capture again.".into());
        }
        if captured.chars().count() > MAX_CAPTURE_CHARS {
            return Err(format!("The selected text is larger than the {MAX_CAPTURE_CHARS}-character capture limit."));
        }
        Ok(captured)
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    use super::{AppControlGrant, AppControlState, AppWindowInfo};
    use tauri::State;
    fn unsupported<T>() -> Result<T,String> { Err("Controlled external-application access is currently implemented for Windows only.".into()) }
    pub fn list_windows() -> Result<Vec<AppWindowInfo>,String> { unsupported() }
    pub fn begin(_:String,_:String,_:State<'_,AppControlState>)->Result<AppControlGrant,String>{ unsupported() }
    pub fn end(_:State<'_,AppControlState>)->Result<(),String>{ unsupported() }
    pub fn status(_:State<'_,AppControlState>)->Result<Option<AppControlGrant>,String>{ Ok(None) }
    pub fn focus(_:State<'_,AppControlState>)->Result<(),String>{ unsupported() }
    pub fn type_text(_:String,_:State<'_,AppControlState>)->Result<(),String>{ unsupported() }
    pub fn type_and_enter(_:String,_:State<'_,AppControlState>)->Result<(),String>{ unsupported() }
    pub fn enter(_:State<'_,AppControlState>)->Result<(),String>{ unsupported() }
    pub fn capture_selection(_:State<'_,AppControlState>)->Result<String,String>{ unsupported() }
}

#[tauri::command]
pub fn app_control_list_windows() -> Result<Vec<AppWindowInfo>, String> { platform::list_windows() }
#[tauri::command]
pub fn app_control_begin(window_id:String, adapter:String, state:tauri::State<'_,AppControlState>) -> Result<AppControlGrant,String> { platform::begin(window_id,adapter,state) }
#[tauri::command]
pub fn app_control_end(state:tauri::State<'_,AppControlState>) -> Result<(),String> { platform::end(state) }
#[tauri::command]
pub fn app_control_status(state:tauri::State<'_,AppControlState>) -> Result<Option<AppControlGrant>,String> { platform::status(state) }
#[tauri::command]
pub fn app_control_focus(state:tauri::State<'_,AppControlState>) -> Result<(),String> { platform::focus(state) }
#[tauri::command]
pub fn app_control_type_text(text:String,state:tauri::State<'_,AppControlState>) -> Result<(),String> { platform::type_text(text,state) }
#[tauri::command]
pub fn app_control_type_and_enter(text:String,state:tauri::State<'_,AppControlState>) -> Result<(),String> { platform::type_and_enter(text,state) }
#[tauri::command]
pub fn app_control_press_enter(state:tauri::State<'_,AppControlState>) -> Result<(),String> { platform::enter(state) }
#[tauri::command]
pub fn app_control_capture_selection(state:tauri::State<'_,AppControlState>) -> Result<String,String> { platform::capture_selection(state) }
