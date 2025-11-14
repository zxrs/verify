#![cfg_attr(not(debug_assertions), windows_subsytem = "windows")]

use anyhow::{Result, ensure};
use std::fmt::Display;
use std::thread;
use windows::{
    Security::Credentials::UI::{
        UserConsentVerificationResult, UserConsentVerifier, UserConsentVerifierAvailability,
    },
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::WinRT::IUserConsentVerifierInterop,
        UI::WindowsAndMessaging::{
            CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW,
            IDI_APPLICATION, LoadCursorW, MB_ICONERROR, MSG, MessageBoxW, PostQuitMessage,
            RegisterClassW, SW_SHOW, SendMessageW, ShowWindow, TranslateMessage, WINDOW_EX_STYLE,
            WM_APP, WM_DESTROY, WNDCLASSW, WS_CAPTION, WS_OVERLAPPED, WS_SYSMENU, WS_VISIBLE,
        },
    },
    core::{HSTRING, PCWSTR, factory, w},
};
use windows_future::IAsyncOperation;

const CLASS_NAME: PCWSTR = w!("verify_window_class");
const WM_VERIFIED: u32 = WM_APP + 1;
const WM_REJECTED: u32 = WM_APP + 2;

struct Hwnd(HWND);

unsafe impl Send for Hwnd {}
unsafe impl Sync for Hwnd {}

fn verify_impl(hwnd: HWND) -> Result<()> {
    let availability = UserConsentVerifier::CheckAvailabilityAsync()?.join()?;
    ensure!(
        availability == UserConsentVerifierAvailability::Available,
        "verfier is not available"
    );

    let verifier = factory::<UserConsentVerifier, IUserConsentVerifierInterop>()?;
    let result = unsafe {
        verifier
            .RequestVerificationForWindowAsync::<IAsyncOperation<UserConsentVerificationResult>>(
                hwnd,
                &"Please verify your identity".into(),
            )?
            .join()?
    };
    ensure!(
        result == UserConsentVerificationResult::Verified,
        "failed to verify"
    );
    Ok(())
}

fn verify(hwnd: Hwnd) {
    match verify_impl(hwnd.0) {
        Ok(_) => unsafe {
            SendMessageW(hwnd.0, WM_VERIFIED, None, None);
        },
        Err(e) => unsafe {
            msg_box(hwnd.0, e);
            SendMessageW(hwnd.0, WM_REJECTED, None, None);
        },
    }
}

fn msg_box(hwnd: HWND, e: impl Display) {
    unsafe {
        MessageBoxW(
            Some(hwnd),
            &HSTRING::from(format!("{e}")),
            w!("Error"),
            MB_ICONERROR,
        );
    };
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_VERIFIED => {
            dbg!("verified");
        }
        WM_REJECTED => {
            dbg!("rejected");
            unsafe {
                PostQuitMessage(1);
            };
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
        }
        _ => return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
    LRESULT::default()
}

fn main() -> Result<()> {
    let wc = WNDCLASSW {
        lpfnWndProc: Some(wnd_proc),
        lpszClassName: CLASS_NAME,
        hCursor: unsafe { LoadCursorW(None, IDI_APPLICATION)? },
        ..Default::default()
    };
    unsafe { RegisterClassW(&wc) };
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            CLASS_NAME,
            w!("verify"),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            300,
            200,
            None,
            None,
            None,
            None,
        )?
    };

    let h = Hwnd(hwnd);
    thread::spawn(move || verify(h));

    _ = unsafe { ShowWindow(hwnd, SW_SHOW) };

    let mut msg = MSG::default();
    loop {
        if unsafe { !GetMessageW(&mut msg, None, 0, 0).as_bool() } {
            break;
        }
        _ = unsafe { TranslateMessage(&msg) };
        unsafe { DispatchMessageW(&msg) };
    }
    Ok(())
}
