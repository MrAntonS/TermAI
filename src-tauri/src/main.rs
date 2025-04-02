#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Mutex;
use std::time::Duration;

use ssh2::{Channel, Session};
use tauri::{AppHandle, Manager, State, Window}; // Added Window
use tokio::sync::mpsc;
use tokio::task;
// Removed unused tokio::io imports for now

// --- State Management ---
struct SshSessionData {
    session: Session,
    channel: Channel,
    stop_sender: Option<mpsc::Sender<()>>,
}

struct AppState {
    ssh_connection: Mutex<Option<SshSessionData>>,
}

impl AppState {
    fn new() -> Self {
        AppState {
            ssh_connection: Mutex::new(None),
        }
    }
}

// --- Event Payloads ---
#[derive(Clone, serde::Serialize)]
struct SshOutputPayload {
    data: String,
}

#[derive(Clone, serde::Serialize)]
struct SshErrorPayload {
    message: String,
}

#[derive(Clone, serde::Serialize)]
struct SshClosedPayload<'a> {
    message: &'a str,
}


// --- Tauri Commands ---

#[tauri::command]
async fn ssh_connect(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    hostname: String,
    port: u16,
    username: String,
    password: Option<String>,
) -> Result<(), String> {
    println!(
        "Attempting SSH connection to {}@{}:{}",
        username, hostname, port
    );

    disconnect_ssh_internal(state.inner())?;

    let tcp = TcpStream::connect(format!("{}:{}", hostname, port))
        .map_err(|e| format!("TCP connection failed: {}", e))?;
    // Set non-blocking *before* passing to Session
    tcp.set_nonblocking(true);
    println!("TCP connection established.");

    let mut sess = Session::new().map_err(|e| format!("Failed to create session: {}", e))?;
    sess.set_tcp_stream(tcp); // Pass the non-blocking stream
    sess.handshake().map_err(|e| format!("SSH handshake failed: {}", e))?;
    // Set session non-blocking *after* handshake and auth might be safer
    sess.set_blocking(false);
    println!("SSH handshake completed, session set to non-blocking.");


    if let Some(pass) = password {
        sess.userauth_password(&username, &pass)
            .map_err(|e| format!("Password authentication failed: {}", e))?;
        println!("Password authentication attempted.");
    } else {
        return Err("Password required (key auth not implemented)".to_string());
    }

    if !sess.authenticated() {
        return Err("Authentication failed".to_string());
    }
    println!("Authentication successful for user '{}'.", username);

    let mut channel = sess
        .channel_session()
        .map_err(|e| format!("Failed to open channel: {}", e))?;
    println!("Channel opened.");

    // Request PTY - Use None for dimensions tuple for defaults
    channel
        .request_pty("xterm-256color", None, None) // Correct signature
        .map_err(|e| format!("Failed to request PTY: {}", e))?;
    println!("PTY requested.");

    channel
        .shell()
        .map_err(|e| format!("Failed to start shell: {}", e))?;
    println!("Shell started.");

    // Channel blocking mode is less relevant when session is non-blocking
    // channel.set_blocking(false); // Not needed/available on Channel

    // --- Spawn Reader Task ---
    let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);
    let mut reader_channel = channel.clone();
    let reader_app_handle = app_handle.clone();

    task::spawn(async move {
        let mut buffer = [0; 4096];
        // Get the main window once for event emission
        let main_window = reader_app_handle.get_webview_window("main");

        loop {
            tokio::select! {
                _ = stop_rx.recv() => {
                    println!("SSH reader task received stop signal.");
                    break;
                }
                result = async { reader_channel.read(&mut buffer) } => {
                    match result {
                        Ok(0) | Ok(_) if reader_channel.eof() => { // Check EOF explicitly
                            println!("SSH channel EOF received or detected.");
                            // if let Some(window) = &main_window {
                            //     let _ = window.emit("ssh-closed", SshClosedPayload { message: "Connection closed by remote" });
                            // } else { eprintln!("Main window not found for ssh-closed event."); }
                            break;
                        }
                        Ok(n) => {
                            match String::from_utf8(buffer[..n].to_vec()) {
                                Ok(data_str) => {
                                    // if let Some(window) = &main_window {
                                    //     if let Err(e) = window.emit("ssh-output", SshOutputPayload { data: data_str }) {
                                    //         eprintln!("Failed to emit ssh-output event: {}", e);
                                    //     }
                                    // } else { eprintln!("Main window not found for ssh-output event."); }
                                }
                                Err(e) => {
                                     eprintln!("SSH received non-UTF8 data: {}", e);
                                    //  if let Some(window) = &main_window {
                                    //     //  let _ = window.emit("ssh-error", SshErrorPayload { message: format!("Received non-UTF8 data: {}", e) });
                                    //  } else { eprintln!("Main window not found for ssh-error event."); }
                                }
                            }
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            // Session is non-blocking, so WouldBlock is expected. Wait before retrying.
                            tokio::time::sleep(Duration::from_millis(10)).await;
                        }
                        Err(e) => {
                            eprintln!("Error reading from SSH channel: {}", e);
                            //  if let Some(window) = &main_window {
                            //     let _ = window.emit("ssh-error", SshErrorPayload { message: format!("Channel read error: {}", e) });
                            //  } else { eprintln!("Main window not found for ssh-error event."); }
                            break; // Exit loop on other errors
                        }
                    }
                }
            }
            // No need for separate EOF check here, handled in match arm
        }
        println!("SSH reader task finished.");
        // Ensure connection is marked as closed if task exits unexpectedly
        if let Some(window) = &main_window {
            //  let _ = window.emit("ssh-closed", SshClosedPayload { message: "Reader task finished" });
        }
    });

    println!("SSH reader task spawned.");

    let mut connection_guard = state.ssh_connection.lock().unwrap();
    *connection_guard = Some(SshSessionData {
        session: sess,
        channel,
        stop_sender: Some(stop_tx),
    });

    println!("SSH connection state stored.");
    Ok(())
}

#[tauri::command]
async fn write_to_ssh(state: State<'_, AppState>, data: String) -> Result<(), String> {
    let mut connection_guard = state.ssh_connection.lock().unwrap();

    if let Some(ref mut session_data) = *connection_guard {
        match session_data.channel.write_all(data.as_bytes()) {
            Ok(_) => {
                 match session_data.channel.flush() {
                     Ok(_) => Ok(()),
                     Err(e) => {
                         eprintln!("Error flushing SSH channel: {}", e);
                         Err(format!("Failed to flush SSH channel: {}", e))
                     }
                 }
            }
            Err(e) => {
                eprintln!("Error writing to SSH channel: {}", e);
                Err(format!("Failed to write to SSH channel: {}", e))
            }
        }
    } else {
        Err("Not connected".to_string())
    }
}

#[tauri::command]
async fn disconnect_ssh(state: State<'_, AppState>) -> Result<(), String> {
    println!("Disconnect command received.");
    disconnect_ssh_internal(state.inner())
}

fn disconnect_ssh_internal(state: &AppState) -> Result<(), String> {
    let mut connection_guard = state.ssh_connection.lock().unwrap();
    if let Some(mut session_data) = connection_guard.take() {
        println!("Closing existing SSH connection...");

        if let Some(sender) = session_data.stop_sender.take() {
            let _ = sender.try_send(()); // Ignore error if receiver dropped
             println!("Stop signal sent to reader task.");
        }

        // Graceful channel closure
        let _ = session_data.channel.send_eof(); // Ignore errors on close
        let _ = session_data.channel.wait_eof();
        let _ = session_data.channel.close();
        let _ = session_data.channel.wait_close();
        println!("SSH channel closed.");

        // Session disconnect happens implicitly when Session is dropped
        println!("SSH connection resources released.");
        Ok(())
    } else {
        println!("No active SSH connection to disconnect.");
        Ok(())
    }
}

// --- Main Application Setup ---
fn main() {
    let app_state = AppState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().build())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            ssh_connect,
            write_to_ssh,
            disconnect_ssh
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
