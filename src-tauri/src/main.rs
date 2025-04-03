#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::io::{self, Read, Write}; // Added io
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread; // Added thread
use std::time::Duration;

use ssh2::{Channel, Session};
use tauri::{AppHandle, Manager, State, Window, Emitter};
use tokio::sync::mpsc::{self, Sender, Receiver}; // Specify mpsc items
use tokio::task;
use tokio::time::interval;

// --- Communication Messages ---
#[derive(Debug)]
enum SshCommand {
    Write(Vec<u8>),
    Disconnect,
}

#[derive(Debug, Clone, serde::Serialize)] // Clone + Serialize for Tauri events
enum SshEvent {
    Data(Vec<u8>), // Send raw bytes, convert to string in frontend or bridge task
    Error(String),
    Closed(String),
}


// --- State Management ---

// Holds communication channels and thread handle for the dedicated SSH I/O thread
struct SshThreadHandle {
    command_sender: Sender<SshCommand>,
    thread_handle: thread::JoinHandle<()>,
}

struct AppState {
    // Option because connection might not exist
    // Arc+Mutex to allow shared access from Tauri commands
    ssh_handle: Arc<Mutex<Option<SshThreadHandle>>>,
}

impl AppState {
    fn new() -> Self {
        AppState {
            ssh_handle: Arc::new(Mutex::new(None)),
        }
    }
}

// --- Event Payloads ---
// #[derive(Clone, serde::Serialize)]
// Payloads are now mostly handled by SshEvent, but keep specific ones if needed
// We'll convert SshEvent::Data to string before emitting if necessary
// --- SSH I/O Thread ---

fn ssh_io_thread(
    mut session: Session, // Takes ownership
    mut channel: Channel, // Takes ownership
    mut command_receiver: Receiver<SshCommand>,
    event_sender: Sender<SshEvent>,
) {
    println!("SSH I/O thread started.");
    let mut read_buf = [0u8; 4096];
    let mut write_buf = Vec::new(); // Buffer for accumulating data from Write commands
    let mut should_disconnect = false;
    let write_interval = Duration::from_millis(50); // Send data roughly every 50ms
    let mut last_write_attempt = std::time::Instant::now();

    // Set channel to non-blocking for reads within the thread loop
    // Session should already be non-blocking from connect logic
    // Channel needs to be non-blocking for both reads and writes
    session.set_blocking(false);

    loop {
        // 1. Check for incoming commands
        match command_receiver.try_recv() {
            Ok(SshCommand::Write(data)) => {
                // Append data to our internal write buffer
                write_buf.extend_from_slice(&data);
                // Don't write immediately, let the write logic below handle it
            }
            Ok(SshCommand::Disconnect) => {
                println!("SSH I/O thread received disconnect command.");
                should_disconnect = true;
                // Don't break immediately, try to flush remaining writes first
            }
            Err(mpsc::error::TryRecvError::Empty) => {
                // No command, continue
            }
            Err(mpsc::error::TryRecvError::Disconnected) => {
                println!("SSH I/O thread command channel disconnected. Shutting down.");
                should_disconnect = true; // Ensure cleanup happens
                break; // Exit loop
            }
        }

        // 2. Check if interval elapsed and buffer has data, then attempt write
        let now = std::time::Instant::now();
        if !write_buf.is_empty() && now.duration_since(last_write_attempt) >= write_interval {
            last_write_attempt = now; // Update time even if write fails/blocks

            match channel.write(&write_buf) {
                 Ok(0) => {
                     // Should not happen with write, indicates an issue
                     let msg = "SSH channel write returned 0 unexpectedly (I/O thread).".to_string();
                     println!("{}", msg);
                     let _ = event_sender.try_send(SshEvent::Error(msg));
                     should_disconnect = true; // Ok(0) is usually fatal for write
                 }
                 Ok(n) => {
                     // Data written, remove it from the buffer
                     //println!("Wrote {} bytes", n); // Debug
                     write_buf.drain(..n);
                 }
                 Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                     // Cannot write right now, buffer is full or busy.
                     // Data remains in write_buf, will retry next interval.
                     //println!("Write would block"); // Debug
                 }
                 Err(e) => {
                     let msg = format!("Error writing to SSH channel (I/O thread): {}", e);
                     println!("{}", msg);
                     let _ = event_sender.try_send(SshEvent::Error(msg));
                     // Don't disconnect immediately on write errors other than Ok(0)
                     should_disconnect = true;
                 }
             }
             // Attempt to flush after writing attempt (regardless of success/block)
             match channel.flush() {
                 Ok(_) => {} // Flush successful or nothing to flush
                 Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {} // Flush would block, okay
                 Err(e) => {
                      let msg = format!("Error flushing SSH channel (I/O thread): {}", e);
                      println!("{}", msg);
                      let _ = event_sender.try_send(SshEvent::Error(msg));
                      // Don't necessarily disconnect on flush error, maybe recoverable
                }
            }
        }

        // 3. Attempt to read data
        match channel.read(&mut read_buf) {
            Ok(0) if channel.eof() => {
                println!("SSH channel EOF detected (I/O thread).");
                let _ = event_sender.try_send(SshEvent::Closed("Connection closed by remote".to_string()));
                should_disconnect = true; // Ensure cleanup
                break; // Exit loop
            }
            Ok(0) => {
                 // Read 0 bytes but not EOF? Should not happen with non-blocking read.
                 // Could indicate a closed channel without EOF. Treat as closed.
                 println!("SSH channel read 0 bytes but not EOF (I/O thread). Treating as closed.");
                 let _ = event_sender.try_send(SshEvent::Closed("Connection closed unexpectedly".to_string()));
                 should_disconnect = true;
                 break;
            }
            Ok(n) => {
                // Send raw bytes back
                let _ = event_sender.try_send(SshEvent::Data(read_buf[..n].to_vec()));
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                // Nothing to read right now
            }
            Err(e) => {
                let msg = format!("Error reading from SSH channel (I/O thread): {}", e);
                println!("{}", msg);
                let _ = event_sender.try_send(SshEvent::Error(msg));
                // Don't disconnect immediately on read errors other than EOF/unexpected close
                should_disconnect = true;
                break;
            }
        }

        // 4. Check if we need to disconnect and exit
        if should_disconnect {
            // If disconnect was requested or an error occurred, exit loop
            break;
        }

        // 5. Prevent busy-looping if nothing happened
        // Only sleep if no commands were received and no read/write occurred (or would block)
        // A more sophisticated approach might use `select!` on channel + command receiver if they were async,
        // but here we are in a sync thread.
        thread::sleep(Duration::from_millis(10)); // Small sleep
    }

    // --- Cleanup ---
    println!("SSH I/O thread shutting down...");
    // Ensure command receiver is drained/closed (happens when `command_receiver` goes out of scope)
    // Ensure event sender is usable until the end (also dropped when out of scope)

    // Attempt graceful shutdown of the SSH channel
    let _ = channel.send_eof();
    let _ = channel.wait_eof(); // These might block briefly
    let _ = channel.close();
    let _ = channel.wait_close(); // These might block briefly
    println!("SSH channel closed (I/O thread).");

    // Session disconnect happens implicitly when `session` is dropped here.
    // We might want to explicitly call session.disconnect, but drop is usually sufficient.
    // let _ = session.disconnect(None, "Disconnecting", None);

    // Send a final closed event if not already sent due to EOF/Error
    // Use try_send as the receiver might have already dropped in the Tokio task
    let _ = event_sender.try_send(SshEvent::Closed("SSH I/O thread finished".to_string()));

    println!("SSH I/O thread finished.");
}


// --- Tauri Commands ---

#[derive(Clone, serde::Serialize)]
struct SshOutputPayload {
    data: String, // Keep this for string conversion
}

#[derive(Clone, serde::Serialize)]
struct SshErrorPayload {
    message: String,
}

#[derive(Clone, serde::Serialize)]
struct SshClosedPayload { // Removed lifetime 'a
    message: String, // Use owned String
}


// --- Tauri Commands ---

#[tauri::command]
async fn ssh_connect(
    app_handle: AppHandle, // Need AppHandle to emit events
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

    // Disconnect any existing session first
    disconnect_ssh_internal(&state).await?; // Pass the State wrapper

    // --- Blocking Network I/O for Setup ---
    // It's often simpler to do the initial connect/auth blocking.
    // Wrap in spawn_blocking if this causes issues in your async context.
    let (sess, channel) = tokio::task::spawn_blocking(move || -> Result<(Session, Channel), String> {
        let tcp = TcpStream::connect(format!("{}:{}", hostname, port))
            .map_err(|e| format!("TCP connection failed: {}", e))?;
        // Keep TCP blocking for setup
        // tcp.set_nonblocking(true).map_err(|e| format!("Failed to set TCP non-blocking: {}", e))?;
        println!("TCP connection established.");

        let mut sess = Session::new().map_err(|e| format!("Failed to create session: {}", e))?;
        sess.set_tcp_stream(tcp);
        sess.handshake().map_err(|e| format!("SSH handshake failed: {}", e))?;
        println!("SSH handshake completed.");


        if let Some(pass) = password {
            sess.userauth_password(&username, &pass)
                .map_err(|e| format!("Password authentication failed: {}", e))?;
            println!("Password authentication successful.");
        } else {
            // Add support for key-based auth here if needed
            return Err("Password authentication is required (key auth not implemented).".to_string());
        }

        if !sess.authenticated() {
            return Err("Authentication failed".to_string());
        }
        println!("Authentication successful for user '{}'.", username);

        // --- Open Channel, Request PTY, Start Shell ---
        let mut channel = sess
            .channel_session()
            .map_err(|e| format!("Failed to open channel: {}", e))?;
        println!("Channel opened.");

        // Request PTY
        channel
            .request_pty("xterm-256color", None, None) // Use appropriate term
            .map_err(|e| format!("Failed to request PTY: {}", e))?;
        println!("PTY requested.");

        // Start Shell
        channel
            .shell()
            .map_err(|e| format!("Failed to start shell: {}", e))?;
        println!("Shell started.");

        // Set session non-blocking *before* spawning thread
        // The thread will set the channel non-blocking internally for reads
        sess.set_blocking(false);
        println!("Session set to non-blocking.");

        Ok((sess, channel))
    }).await.map_err(|e| format!("Blocking task failed: {}", e))??; // Join + unwrap Result

    // --- Setup Communication Channels ---
    let (command_tx, command_rx) = mpsc::channel::<SshCommand>(32); // Channel for sending commands to I/O thread
    let (event_tx, mut event_rx) = mpsc::channel::<SshEvent>(128); // Channel for receiving events from I/O thread

    // --- Spawn Dedicated I/O Thread ---
    let thread_handle = thread::spawn(move || {
        // This thread owns session and channel now
        ssh_io_thread(sess, channel, command_rx, event_tx);
    });
    println!("SSH I/O thread spawned.");

    // --- Spawn Tokio Task to Bridge Events to Tauri ---
    let event_app_handle = app_handle.clone();
    let event_state_clone = Arc::clone(&state.ssh_handle); // Clone Arc<Mutex<Option<SshThreadHandle>>>
    task::spawn(async move {
        println!("Tauri event bridge task started.");
        while let Some(event) = event_rx.recv().await {
            let main_window = event_app_handle.get_webview_window("main");
            if main_window.is_none() {
                 eprintln!("Event bridge: Main window not found. Cannot emit event: {:?}", event);
                 continue;
            }
            let window = main_window.unwrap();

            match event {
                SshEvent::Data(bytes) => {
                    // Attempt to convert to UTF-8, send error if fails
                    match String::from_utf8(bytes) {
                        Ok(data_str) => {
                            if let Err(e) = window.emit("ssh-output", SshOutputPayload { data: data_str }) {
                                eprintln!("Failed to emit ssh-output event: {}", e);
                            }
                        }
                        Err(e) => {
                            let msg = format!("Received non-UTF8 data: {}", e);
                            eprintln!("{}", msg);
                            let _ = window.emit("ssh-error", SshErrorPayload { message: msg });
                        }
                    }
                }
                SshEvent::Error(msg) => {
                    eprintln!("SSH Error Event: {}", msg);
                    let _ = window.emit("ssh-error", SshErrorPayload { message: msg });
                }
                SshEvent::Closed(msg) => {
                    println!("SSH Closed Event: {}", msg);
                    let _ = window.emit("ssh-closed", SshClosedPayload { message: msg.clone() }); // Clone msg

                    // Important: Since the connection is closed from the I/O thread's perspective,
                    // we should ensure the state in AppState reflects this.
                    // We attempt to remove the handle here. This might race with explicit disconnect,
                    // but helps clean up if the connection drops unexpectedly.
                    println!("Event bridge attempting to clear state due to Closed event.");
                    if let Ok(mut guard) = event_state_clone.lock() {
                        if let Some(handle_data) = guard.take() {
                             println!("SSH handle removed from state by event bridge.");
                             // We don't join the thread here, just remove the handle
                        } else {
                             println!("State already cleared when processing Closed event.");
                        }
                    } else {
                        eprintln!("Event bridge failed to lock state to clear handle after Closed event.");
                    }
                    break; // Stop the event bridge task as the connection is closed
                }
            }
        }
        println!("Tauri event bridge task finished.");
    });
    println!("Tauri event bridge task spawned.");

    // --- Store Handle in State ---
    { // Scope for mutex guard
        let mut handle_guard = state.ssh_handle.lock().map_err(|_| "Failed to lock state mutex".to_string())?;
        *handle_guard = Some(SshThreadHandle {
            command_sender: command_tx,
            thread_handle,
        });
    } // Mutex guard dropped here

    println!("SSH connection process completed successfully.");
    Ok(())
}

#[tauri::command]
async fn write_to_ssh(state: State<'_, AppState>, data: String) -> Result<(), String> {
    let command_sender = { // Scope to hold the lock guard briefly
        let guard = state.ssh_handle.lock().map_err(|_| "Failed to lock state mutex".to_string())?;
        // Clone the sender if a connection exists
        guard.as_ref().map(|handle| handle.command_sender.clone())
    };

    if let Some(sender) = command_sender {
        // Send the write command asynchronously
        sender
            .send(SshCommand::Write(data.into_bytes()))
            .await
            .map_err(|e| format!("Failed to send write command: {}", e))
    } else {
        Err("Not connected".to_string())
    }
}

#[tauri::command]
async fn disconnect_ssh(state: State<'_, AppState>) -> Result<(), String> {
    println!("Disconnect command received.");
    // Call the internal async function, passing the State directly
    disconnect_ssh_internal(&state).await
}

// Make internal function async as it interacts with async channels and potentially waits
async fn disconnect_ssh_internal(state: &AppState) -> Result<(), String> {
    println!("Attempting to disconnect SSH...");

    // 1. Lock the state and take the handle
    let handle_to_disconnect = { // Scope for mutex guard
        let mut guard = state.ssh_handle.lock().map_err(|_| "Failed to lock state mutex for disconnect".to_string())?;
        guard.take() // Takes the Option<SshThreadHandle>, leaving None
    };

    // 2. If a handle exists, send disconnect command and join the thread
    if let Some(handle) = handle_to_disconnect {
        println!("Found active SSH handle. Sending disconnect command...");

        // Send the disconnect command (fire and forget, thread will handle cleanup)
        // Ignore error if channel is already closed (thread might have terminated)
        let _ = handle.command_sender.send(SshCommand::Disconnect).await;
        println!("Disconnect command sent to I/O thread.");

        // 3. Wait for the I/O thread to finish (blocking operation)
        // Use spawn_blocking to avoid blocking the async runtime
        println!("Waiting for SSH I/O thread to join...");
        let join_result = task::spawn_blocking(move || {
            handle.thread_handle.join()
        }).await;

        match join_result {
            Ok(Ok(_)) => {
                println!("SSH I/O thread joined successfully.");
                Ok(())
            }
            Ok(Err(e)) => {
                // The thread panicked
                eprintln!("SSH I/O thread panicked: {:?}", e);
                Err("SSH I/O thread panicked during shutdown.".to_string())
            }
            Err(e) => {
                // The spawn_blocking task itself failed (rare)
                eprintln!("Failed to join SSH I/O thread task: {}", e);
                Err(format!("Failed to join SSH I/O thread: {}", e))
            }
        }
    } else {
        println!("No active SSH connection found to disconnect.");
        Ok(()) // No connection, so disconnect is trivially successful
    }
}

// --- Main Application Setup ---
fn main() {
    let app_state = AppState::new(); // Creates Arc<Mutex<Option<SshSessionData>>>

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
