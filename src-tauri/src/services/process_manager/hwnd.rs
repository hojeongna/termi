use std::collections::HashSet;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows_sys::Win32::System::Diagnostics::ToolHelp::*;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use super::types::*;

/// "termi: {project_name}" 제목을 가진 Windows Terminal 창의 HWND를 찾는다
pub(super) fn find_terminal_hwnd(title_prefix: &str) -> Option<isize> {
    struct SearchData {
        prefix: Vec<u16>,
        found_hwnd: isize,
    }

    unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: lparam points to a valid SearchData on the caller's stack; its lifetime outlives this callback.
        let data = &mut *(lparam as *mut SearchData);

        if IsWindowVisible(hwnd) == 0 {
            return TRUE;
        }

        let title_len = GetWindowTextLengthW(hwnd);
        if title_len <= 0 {
            return TRUE;
        }

        let mut title_buf = vec![0u16; (title_len + 1) as usize];
        GetWindowTextW(hwnd, title_buf.as_mut_ptr(), title_buf.len() as i32);

        if title_buf.len() >= data.prefix.len()
            && title_buf[..data.prefix.len()] == data.prefix[..]
        {
            data.found_hwnd = hwnd as isize;
            return FALSE;
        }

        TRUE
    }

    let prefix_utf16: Vec<u16> = title_prefix.encode_utf16().collect();
    let mut data = SearchData {
        prefix: prefix_utf16,
        found_hwnd: 0,
    };

    // SAFETY: search_data is valid for the duration of EnumWindows; callback correctly accesses it via lparam.
    unsafe {
        EnumWindows(Some(enum_callback), &mut data as *mut _ as isize);
    }

    if data.found_hwnd != 0 {
        Some(data.found_hwnd)
    } else {
        None
    }
}

/// HWND의 윈도우 제목을 읽어 반환한다. 실패 시 빈 문자열.
pub(super) fn get_window_title(hwnd: HWND) -> String {
    // SAFETY: GetWindowTextLengthW is safe with any HWND value.
    let len = unsafe { GetWindowTextLengthW(hwnd) };
    if len <= 0 {
        return String::new();
    }
    let mut buf = vec![0u16; (len + 1) as usize];
    // SAFETY: buf is allocated with sufficient capacity (len + 1) for the window text.
    let actual = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    if actual <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..actual as usize])
}

/// UIA를 사용하여 Windows Terminal 창의 개별 탭 (RuntimeId, 제목) 목록을 반환한다.
pub(super) fn get_tab_items_via_uia(
    automation: &uiautomation::UIAutomation,
    hwnd: isize,
) -> Vec<(Vec<i32>, String)> {
    let element = match automation.element_from_handle(uiautomation::types::Handle::from(hwnd)) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let matcher = automation.create_matcher()
        .from(element)
        .control_type(uiautomation::types::ControlType::TabItem)
        .timeout(UIA_MATCHER_TIMEOUT_MS)
        .depth(UIA_SEARCH_DEPTH);

    match matcher.find_all() {
        Ok(items) => items.iter()
            .filter_map(|item| {
                let rid = item.get_runtime_id().ok()?;
                let name = item.get_name().unwrap_or_default();
                Some((rid, name))
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// UIA를 사용하여 RuntimeId로 특정 탭을 선택한다.
pub(super) fn select_tab_via_uia(hwnd: isize, target_rid: &[i32]) {
    let automation = match uiautomation::UIAutomation::new() {
        Ok(a) => a,
        Err(_) => return,
    };

    let tabs = get_tab_items_via_uia(&automation, hwnd);
    for (idx, (rid, _name)) in tabs.iter().enumerate() {
        if rid == target_rid {
            // UIA 탭 요소를 다시 찾아서 SelectionItemPattern으로 선택
            let element = match automation.element_from_handle(uiautomation::types::Handle::from(hwnd)) {
                Ok(e) => e,
                Err(_) => return,
            };
            let matcher = automation.create_matcher()
                .from(element)
                .control_type(uiautomation::types::ControlType::TabItem)
                .timeout(UIA_MATCHER_TIMEOUT_MS)
                .depth(UIA_SEARCH_DEPTH);

            if let Ok(items) = matcher.find_all() {
                if let Some(item) = items.get(idx) {
                    if let Ok(pattern) = item.get_pattern::<uiautomation::patterns::UISelectionItemPattern>() {
                        let _ = pattern.select();
                    }
                }
            }
            return;
        }
    }
}

/// HWND의 Windows Terminal 창을 포그라운드로 이동
pub(super) fn focus_window(hwnd: HWND) -> Result<(), crate::error::AppError> {
    // SAFETY: These Win32 functions are safe to call with any HWND; they fail gracefully for invalid handles.
    unsafe {
        ShowWindow(hwnd, SW_RESTORE);
        if SetForegroundWindow(hwnd) == 0 {
            return Err(crate::error::AppError::Terminal("포그라운드 전환 실패".to_string()));
        }
    }
    Ok(())
}

/// UIA를 사용하여 HWND의 탭 개수를 반환한다.
pub(super) fn count_tabs_via_uia(hwnd: isize) -> usize {
    let automation = match uiautomation::UIAutomation::new() {
        Ok(a) => a,
        Err(_) => return 0,
    };
    get_tab_items_via_uia(&automation, hwnd).len()
}

/// EnumWindows로 모든 가시 윈도우를 열거하고 Windows Terminal 클래스인 것만 반환한다.
pub(super) fn enumerate_wt_windows() -> Vec<isize> {
    struct CollectData {
        hwnds: Vec<isize>,
    }

    unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: lparam points to a valid CollectData on the caller's stack.
        let data = &mut *(lparam as *mut CollectData);

        if IsWindowVisible(hwnd) == 0 {
            return TRUE;
        }

        let mut class_name = [0u16; 256];
        let len = GetClassNameW(hwnd, class_name.as_mut_ptr(), 256);
        if len <= 0 {
            return TRUE;
        }
        let name = String::from_utf16_lossy(&class_name[..len as usize]);
        if name == WT_WINDOW_CLASS {
            data.hwnds.push(hwnd as isize);
        }

        TRUE
    }

    let mut data = CollectData { hwnds: Vec::new() };
    // SAFETY: data is valid for the duration of EnumWindows; callback correctly accesses it via lparam.
    unsafe {
        EnumWindows(Some(enum_callback), &mut data as *mut _ as isize);
    }

    data.hwnds
}

/// HWND에서 PID를 가져온다.
pub(super) fn get_pid_from_hwnd(hwnd: isize) -> u32 {
    let mut pid: u32 = 0;
    // SAFETY: GetWindowThreadProcessId is safe with any HWND value.
    unsafe { GetWindowThreadProcessId(hwnd as HWND, &mut pid); }
    pid
}

/// 지정된 PID의 프로세스의 현재 작업 디렉토리(cwd)를 읽는다.
pub(super) fn get_process_cwd(pid: u32) -> Option<String> {
    // SAFETY: handle from OpenProcess is checked for null before use, CloseHandle is called to release it.
    unsafe {
        // 프로세스 열기
        let handle = OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
            FALSE,
            pid,
        );
        if handle.is_null() {
            return None;
        }

        let result = read_cwd_from_handle(handle);
        CloseHandle(handle);
        result
    }
}

/// 프로세스 핸들로부터 PEB를 읽어 cwd를 추출한다.
unsafe fn read_cwd_from_handle(handle: HANDLE) -> Option<String> {
    // 1. PEB 주소 얻기
    // SAFETY: ProcessBasicInformation is a repr(C) struct of integers and pointers,
    // which is valid when zero-initialized (all fields are numeric types).
    let mut pbi = std::mem::zeroed::<ProcessBasicInformation>();
    // SAFETY: handle is a valid process handle opened with PROCESS_QUERY_INFORMATION by the caller.
    // pbi is a correctly-sized local buffer matching the ProcessBasicInformation layout.
    let status = NtQueryInformationProcess(
        handle,
        0, // ProcessBasicInformation
        &mut pbi as *mut _ as *mut std::ffi::c_void,
        std::mem::size_of::<ProcessBasicInformation>() as u32,
        std::ptr::null_mut(),
    );
    if status != 0 || pbi.peb_base_address == 0 {
        return None;
    }

    // 2. PEB에서 ProcessParameters 포인터 읽기 (offset 0x20 on x64)
    let params_ptr_addr = pbi.peb_base_address + PEB_PROCESS_PARAMETERS_OFFSET;
    let mut params_ptr: usize = 0;
    // SAFETY: handle is valid with PROCESS_VM_READ access. params_ptr_addr is the PEB address
    // (obtained from NtQueryInformationProcess) plus the documented x64 offset for ProcessParameters.
    // We read exactly size_of::<usize>() bytes into a local usize variable.
    let ok = ReadProcessMemory(
        handle,
        params_ptr_addr as *const std::ffi::c_void,
        &mut params_ptr as *mut _ as *mut std::ffi::c_void,
        std::mem::size_of::<usize>(),
        std::ptr::null_mut(),
    );
    if ok == 0 || params_ptr == 0 {
        return None;
    }

    // 3. RTL_USER_PROCESS_PARAMETERS에서 CurrentDirectory.DosPath 읽기
    //    offset 0x38 on x64: UNICODE_STRING { Length: u16, MaxLength: u16, _pad: u32, Buffer: *u16 }
    let cwd_unicode_addr = params_ptr + PROCESS_PARAMS_CWD_OFFSET;
    let mut length: u16 = 0;
    // SAFETY: handle is valid with PROCESS_VM_READ. cwd_unicode_addr points to the UNICODE_STRING.Length
    // field within RTL_USER_PROCESS_PARAMETERS (obtained from the validated ProcessParameters pointer).
    // We read exactly size_of::<u16>() bytes into a local u16 variable.
    let ok = ReadProcessMemory(
        handle,
        cwd_unicode_addr as *const std::ffi::c_void,
        &mut length as *mut _ as *mut std::ffi::c_void,
        std::mem::size_of::<u16>(),
        std::ptr::null_mut(),
    );
    if ok == 0 || length == 0 {
        return None;
    }

    // Buffer pointer is at offset +8 (after Length u16, MaxLength u16, padding u32)
    let buffer_ptr_addr = cwd_unicode_addr + UNICODE_STRING_BUFFER_PTR_OFFSET;
    let mut buffer_ptr: usize = 0;
    // SAFETY: handle is valid with PROCESS_VM_READ. buffer_ptr_addr points to the UNICODE_STRING.Buffer
    // pointer field at the documented offset within the same UNICODE_STRING struct.
    // We read exactly size_of::<usize>() bytes into a local usize variable.
    let ok = ReadProcessMemory(
        handle,
        buffer_ptr_addr as *const std::ffi::c_void,
        &mut buffer_ptr as *mut _ as *mut std::ffi::c_void,
        std::mem::size_of::<usize>(),
        std::ptr::null_mut(),
    );
    if ok == 0 || buffer_ptr == 0 {
        return None;
    }

    // 4. 유니코드 문자열 버퍼 읽기
    let char_count = (length as usize) / 2;
    let mut buf = vec![0u16; char_count];
    // SAFETY: handle is valid with PROCESS_VM_READ. buffer_ptr is the address read from the
    // UNICODE_STRING.Buffer field, and length is the byte count from UNICODE_STRING.Length.
    // buf is allocated with exactly char_count = length/2 u16 elements, matching the byte size.
    let ok = ReadProcessMemory(
        handle,
        buffer_ptr as *const std::ffi::c_void,
        buf.as_mut_ptr() as *mut std::ffi::c_void,
        length as usize,
        std::ptr::null_mut(),
    );
    if ok == 0 {
        return None;
    }

    let path = String::from_utf16_lossy(&buf);
    // 뒤의 '\' 제거
    Some(path.trim_end_matches('\\').to_string())
}

/// 지정된 부모 PID의 모든 자손 프로세스 중 셸 프로세스의 cwd를 반환한다.
pub(super) fn get_descendant_shell_cwds(parent_pid: u32) -> Vec<String> {
    let all_procs = enumerate_processes();
    let descendant_pids = collect_descendants(parent_pid, &all_procs);

    let mut cwds = Vec::new();
    for (pid, exe_name) in &descendant_pids {
        let exe_lower = exe_name.to_lowercase();
        if SHELL_EXECUTABLES.iter().any(|s| exe_lower == *s) {
            if let Some(cwd) = get_process_cwd(*pid) {
                if !cwd.is_empty() && !cwds.iter().any(|c: &String| c.to_lowercase() == cwd.to_lowercase()) {
                    cwds.push(cwd);
                }
            }
        }
    }
    cwds
}

/// 모든 프로세스를 열거하여 (pid, parent_pid, exe_name) 목록을 반환한다.
fn enumerate_processes() -> Vec<(u32, u32, String)> {
    let mut result = Vec::new();
    // SAFETY: PROCESSENTRY32W is a C struct with valid all-zero representation,
    // snapshot handle is checked against INVALID_HANDLE_VALUE, CloseHandle called unconditionally.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return result;
        }

        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                let exe_name = String::from_utf16_lossy(
                    &entry.szExeFile[..entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len())]
                );
                result.push((entry.th32ProcessID, entry.th32ParentProcessID, exe_name));

                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }

        CloseHandle(snapshot);
    }
    result
}

/// 부모 PID에서 시작하여 모든 자손 프로세스 (pid, exe_name)을 수집한다.
fn collect_descendants(parent_pid: u32, all_procs: &[(u32, u32, String)]) -> Vec<(u32, String)> {
    let mut result = Vec::new();
    let mut queue = vec![parent_pid];
    let mut visited = HashSet::new();

    while let Some(pid) = queue.pop() {
        if !visited.insert(pid) {
            continue;
        }
        for (child_pid, parent, exe_name) in all_procs {
            if *parent == pid && *child_pid != pid {
                result.push((*child_pid, exe_name.clone()));
                queue.push(*child_pid);
            }
        }
    }
    result
}

/// WT 윈도우의 자손 셸 프로세스 cwd 목록을 반환한다.
pub(super) fn get_terminal_cwds(hwnd: isize) -> Vec<String> {
    let pid = get_pid_from_hwnd(hwnd);
    if pid == 0 {
        return Vec::new();
    }
    get_descendant_shell_cwds(pid)
}

#[cfg(test)]
mod tests {
    use super::{find_terminal_hwnd, get_process_cwd, get_descendant_shell_cwds};
    use super::super::types::WT_WINDOW_CLASS;

    #[test]
    fn find_terminal_hwnd_returns_none_for_nonexistent_title() {
        assert!(find_terminal_hwnd("termi: this-window-should-not-exist-12345").is_none());
    }

    #[test]
    fn get_process_cwd_returns_cwd_for_current_process() {
        let pid = std::process::id();
        let cwd = get_process_cwd(pid);
        assert!(cwd.is_some(), "should be able to read current process cwd");
        let cwd_str = cwd.unwrap();
        assert!(!cwd_str.is_empty());
    }

    #[test]
    fn get_descendant_shell_cwds_returns_vec() {
        // Use current process PID — may or may not have shell descendants
        let pid = std::process::id();
        let cwds = get_descendant_shell_cwds(pid);
        // Just verify it doesn't crash; may be empty
        assert!(cwds.len() >= 0);
    }

    #[test]
    fn wt_window_class_constant_is_defined() {
        assert_eq!(WT_WINDOW_CLASS, "CASCADIA_HOSTING_WINDOW_CLASS");
    }
}
