use crate::clipboard;
use serde::Serialize;
use std::{collections::BTreeSet, mem::size_of};
use tauri::AppHandle;
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{CloseHandle, BOOL, ERROR_BUFFER_OVERFLOW, HANDLE, HWND, NO_ERROR},
        NetworkManagement::{
            IpHelper::{
                GetAdaptersAddresses, GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER,
                GAA_FLAG_SKIP_MULTICAST, IP_ADAPTER_ADDRESSES_LH,
            },
            Ndis::IfOperStatusUp,
        },
        Networking::WinSock::{AF_INET, SOCKADDR_IN},
        Security::{
            AdjustTokenPrivileges, LookupPrivilegeValueW, LUID_AND_ATTRIBUTES,
            SE_PRIVILEGE_ENABLED, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
        },
        System::{
            Com::CoCreateGuid,
            Power::SetSuspendState,
            Shutdown::{
                InitiateSystemShutdownExW, LockWorkStation, SHTDN_REASON_FLAG_PLANNED,
                SHTDN_REASON_MAJOR_APPLICATION, SHTDN_REASON_MINOR_OTHER,
            },
            Threading::{GetCurrentProcess, OpenProcessToken},
        },
        UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
    },
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickActionResponse {
    message: String,
    values: Vec<String>,
}

impl QuickActionResponse {
    fn message(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            values: Vec::new(),
        }
    }
}

#[tauri::command]
pub async fn execute_quick_action(
    app: AppHandle,
    action: String,
    argument: Option<String>,
) -> Result<QuickActionResponse, String> {
    match action.as_str() {
        "uuid" => {
            let value = unsafe { CoCreateGuid() }.map_err(|error| error.to_string())?;
            let value = format!("{value:?}").to_lowercase();
            clipboard::copy_text(app, value).await?;
            Ok(QuickActionResponse::message("i18n:backend.uuidCopied"))
        }
        "color" => {
            let rgb = hex_to_rgb(argument.as_deref().unwrap_or_default())
                .ok_or_else(|| "i18n:command.invalidColor".to_string())?;
            clipboard::copy_text(app, rgb).await?;
            Ok(QuickActionResponse::message("i18n:backend.rgbCopied"))
        }
        "ip" => {
            let values = tauri::async_runtime::spawn_blocking(local_ipv4_addresses)
                .await
                .map_err(|error| error.to_string())??;
            if values.is_empty() {
                return Err("i18n:backend.noIp".into());
            }
            Ok(QuickActionResponse {
                message: "i18n:backend.selectIp".into(),
                values,
            })
        }
        "lock" | "sleep" | "restart" | "shutdown" | "recycle" => {
            let action = action.clone();
            tauri::async_runtime::spawn_blocking(move || run_windows_action(&action))
                .await
                .map_err(|error| error.to_string())??;
            Ok(QuickActionResponse::message("i18n:backend.commandDone"))
        }
        _ => Err("i18n:backend.unknownCommand".into()),
    }
}

#[tauri::command]
pub async fn copy_command_value(
    app: AppHandle,
    value: String,
) -> Result<QuickActionResponse, String> {
    if value.len() > 256 {
        return Err("i18n:backend.valueTooLong".into());
    }
    clipboard::copy_text(app, value).await?;
    Ok(QuickActionResponse::message("i18n:backend.copied"))
}

fn hex_to_rgb(value: &str) -> Option<String> {
    let value = value.trim().strip_prefix('#').unwrap_or(value.trim());
    let expanded;
    let value = if value.len() == 3 {
        if !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        expanded = value
            .chars()
            .flat_map(|character| [character, character])
            .collect::<String>();
        expanded.as_str()
    } else {
        value
    };
    if value.len() != 6 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let number = u32::from_str_radix(value, 16).ok()?;
    Some(format!(
        "rgb({}, {}, {})",
        number >> 16,
        (number >> 8) & 255,
        number & 255
    ))
}

fn run_windows_action(action: &str) -> Result<(), String> {
    unsafe {
        match action {
            "lock" => LockWorkStation().map_err(|error| error.to_string()),
            "sleep" => {
                if SetSuspendState(false, false, false).as_bool() {
                    Ok(())
                } else {
                    Err("i18n:backend.sleepDenied".into())
                }
            }
            "restart" => shutdown(true),
            "shutdown" => shutdown(false),
            "recycle" => {
                let result = ShellExecuteW(
                    HWND::default(),
                    w!("open"),
                    w!("shell:RecycleBinFolder"),
                    PCWSTR::null(),
                    PCWSTR::null(),
                    SW_SHOWNORMAL,
                );
                if result.0 as isize <= 32 {
                    Err("i18n:backend.recycleFailed".into())
                } else {
                    Ok(())
                }
            }
            _ => Err("i18n:backend.unknownCommand".into()),
        }
    }
}

unsafe fn shutdown(reboot: bool) -> Result<(), String> {
    enable_shutdown_privilege()?;
    let reason =
        SHTDN_REASON_MAJOR_APPLICATION | SHTDN_REASON_MINOR_OTHER | SHTDN_REASON_FLAG_PLANNED;
    InitiateSystemShutdownExW(
        PCWSTR::null(),
        w!("SeekDash hızlı komutu"),
        0,
        BOOL(0),
        BOOL(reboot as i32),
        reason,
    )
    .map_err(|error| error.to_string())
}

unsafe fn enable_shutdown_privilege() -> Result<(), String> {
    let mut token = HANDLE::default();
    OpenProcessToken(
        GetCurrentProcess(),
        TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
        &mut token,
    )
    .map_err(|error| error.to_string())?;
    let result = (|| {
        let mut luid = Default::default();
        LookupPrivilegeValueW(PCWSTR::null(), w!("SeShutdownPrivilege"), &mut luid)
            .map_err(|error| error.to_string())?;
        let privileges = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: SE_PRIVILEGE_ENABLED,
            }],
        };
        AdjustTokenPrivileges(token, false, Some(&privileges), 0, None, None)
            .map_err(|error| error.to_string())
    })();
    let _ = CloseHandle(token);
    result
}

fn local_ipv4_addresses() -> Result<Vec<String>, String> {
    unsafe {
        let flags = GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_DNS_SERVER;
        let mut length = 0u32;
        let first = GetAdaptersAddresses(AF_INET.0 as u32, flags, None, None, &mut length);
        if first != ERROR_BUFFER_OVERFLOW.0 {
            return Err(format!("Ağ arayüzleri okunamadı ({first})"));
        }
        let mut buffer = vec![0u8; length as usize + size_of::<IP_ADAPTER_ADDRESSES_LH>()];
        let head = buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH;
        let status = GetAdaptersAddresses(AF_INET.0 as u32, flags, None, Some(head), &mut length);
        if status != NO_ERROR.0 {
            return Err(format!("Ağ arayüzleri okunamadı ({status})"));
        }
        let mut addresses = BTreeSet::new();
        let mut adapter = head;
        while !adapter.is_null() {
            if (*adapter).OperStatus == IfOperStatusUp {
                let mut unicast = (*adapter).FirstUnicastAddress;
                while !unicast.is_null() {
                    let socket = (*unicast).Address.lpSockaddr;
                    if !socket.is_null() && (*socket).sa_family == AF_INET {
                        let address = socket as *const SOCKADDR_IN;
                        let bytes = (*address).sin_addr.S_un.S_addr.to_ne_bytes();
                        if bytes[0] != 127 && bytes != [0, 0, 0, 0] {
                            addresses.insert(format!(
                                "{}.{}.{}.{}",
                                bytes[0], bytes[1], bytes[2], bytes[3]
                            ));
                        }
                    }
                    unicast = (*unicast).Next;
                }
            }
            adapter = (*adapter).Next;
        }
        Ok(addresses.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::hex_to_rgb;

    #[test]
    fn converts_hex_colors_safely() {
        assert_eq!(hex_to_rgb("#FF5733").as_deref(), Some("rgb(255, 87, 51)"));
        assert_eq!(hex_to_rgb("0af").as_deref(), Some("rgb(0, 170, 255)"));
        assert_eq!(hex_to_rgb("#abcd"), None);
        assert_eq!(hex_to_rgb("not-a-color"), None);
    }
}
