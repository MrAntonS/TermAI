#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::io::{Read, Write}; // Removed BufReader, BufRead
use std::process::{Command, Stdio, Child, ChildStdin, ChildStdout, ChildStderr};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager, State, Window, Emitter};
use tokio::sync::mpsc::{self, Sender, Receiver};
use tokio::task;

mod gemini_api; // Add the new module

// --- Communication Messages ---
#[derive(Debug)]
enum SshCommand {
    Write(Vec<u8>),
    Disconnect,
}

// --- Event Payloads --- Keep these as they define the frontend contract
#[derive(Clone, serde::Serialize)]
struct SshOutputPayload {
    data: String,
}

#[derive(Clone, serde::Serialize)]
struct SshErrorPayload {
    message: String,
}

#[derive(Clone, serde::Serialize)]
struct SshClosedPayload {
    message: String,
}


// --- State Management ---

// Holds the running process handle and communication channel
struct SshProcessHandle {
    child: Arc<Mutex<Child>>, // Arc<Mutex<>> for shared access to kill
    stdin: Arc<Mutex<ChildStdin>>, // Arc<Mutex<>> for shared access to write
    command_sender: Sender<SshCommand>, // To send write/disconnect commands
}

struct AppState {
    ssh_handle: Arc<Mutex<Option<SshProcessHandle>>>,
}

impl AppState {
    fn new() -> Self {
        AppState {
            ssh_handle: Arc::new(Mutex::new(None)),
        }
    }
}

// --- Helper function to emit events ---
fn emit_event<P: serde::Serialize + Clone>(app_handle: &AppHandle, event: &str, payload: P) {
    if let Some(window) = app_handle.get_webview_window("main") {
        if let Err(e) = window.emit(event, payload) {
            eprintln!("Failed to emit event '{}': {}", event, e);
        }
    } else {
         eprintln!("Event emit failed: Main window not found for event '{}'.", event);
    }
}

// --- I/O Handling Threads/Tasks ---

// Task to read stdout and emit events (using raw bytes)
fn spawn_stdout_reader(app_handle: AppHandle, mut stdout: ChildStdout) {
    thread::spawn(move || {
        println!("SSH stdout reader thread started.");
        let mut buffer = [0; 4096]; // Read in chunks
        loop {
            match stdout.read(&mut buffer) {
                Ok(0) => {
                    // EOF reached
                    println!("SSH stdout EOF reached.");
                    break;
                }
                Ok(n) => {
                    // Successfully read n bytes
                    // Attempt to convert to UTF-8. Handle invalid sequences gracefully.
                    let data_str = String::from_utf8_lossy(&buffer[..n]).to_string();
                    emit_event(&app_handle, "ssh-output", SshOutputPayload { data: data_str });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {
                    // Interrupted by signal, try again
                    continue;
                }
                Err(e) => {
                    // Other read error
                    let msg = format!("Error reading SSH stdout: {}", e);
                    eprintln!("{}", msg);
                    emit_event(&app_handle, "ssh-error", SshErrorPayload { message: msg });
                    break;
                }
            }
        }
        println!("SSH stdout reader thread finished.");
        // Optionally emit closed event here if needed, though wait_handler covers process exit
    });
}

// Task to read stderr and emit events (using raw bytes)
fn spawn_stderr_reader(app_handle: AppHandle, mut stderr: ChildStderr) {
    thread::spawn(move || {
        println!("SSH stderr reader thread started.");
        let mut buffer = [0; 1024]; // Smaller buffer for stderr often okay
        loop {
            match stderr.read(&mut buffer) {
                Ok(0) => {
                    // EOF reached
                    println!("SSH stderr EOF reached.");
                    break;
                }
                Ok(n) => {
                    // Successfully read n bytes
                    let error_msg = String::from_utf8_lossy(&buffer[..n]).to_string();
                    let msg = format!("SSH stderr: {}", error_msg);
                    eprintln!("{}", msg); // Log locally
                    emit_event(&app_handle, "ssh-error", SshErrorPayload { message: msg });
                }
                 Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {
                    // Interrupted by signal, try again
                    continue;
                }
                Err(e) => {
                    // Other read error
                    let msg = format!("Error reading SSH stderr: {}", e);
                    eprintln!("{}", msg);
                    // Optionally emit error event, but might be redundant if process exits
                    // emit_event(&app_handle, "ssh-error", SshErrorPayload { message: msg });
                    break;
                }
            }
        }
        println!("SSH stderr reader thread finished.");
    });
}

// Task to handle commands (Write, Disconnect)
fn spawn_command_handler(
    app_handle: AppHandle,
    state_clone: Arc<Mutex<Option<SshProcessHandle>>>,
    mut command_receiver: Receiver<SshCommand>,
    child_arc: Arc<Mutex<Child>>,
    stdin_arc: Arc<Mutex<ChildStdin>>,
) {
    tokio::spawn(async move {
        println!("SSH command handler task started.");
        while let Some(command) = command_receiver.recv().await {
            match command {
                SshCommand::Write(data) => {
                    let mut stdin_guard = stdin_arc.lock().unwrap();
                    if let Err(e) = stdin_guard.write_all(&data) {
                        let msg = format!("Error writing to SSH stdin: {}", e);
                        eprintln!("{}", msg);
                        emit_event(&app_handle, "ssh-error", SshErrorPayload { message: msg });
                    } else {
                         if let Err(e) = stdin_guard.flush() {
                             let msg = format!("Error flushing SSH stdin: {}", e);
                             eprintln!("{}", msg);
                             emit_event(&app_handle, "ssh-error", SshErrorPayload { message: msg });
                         }
                    }
                    drop(stdin_guard);
                }
                SshCommand::Disconnect => {
                    println!("Command handler received disconnect.");
                    let mut child_guard = child_arc.lock().unwrap();
                    if let Err(e) = child_guard.kill() {
                        let msg = format!("Failed to kill SSH process: {}", e);
                        eprintln!("{}", msg);
                        emit_event(&app_handle, "ssh-error", SshErrorPayload { message: msg });
                    } else {
                        println!("SSH process kill signal sent.");
                    }
                    drop(child_guard);

                    let mut state_guard = state_clone.lock().unwrap();
                    if state_guard.is_some() {
                        *state_guard = None;
                        println!("SSH handle removed from state by command handler.");
                    }
                    break;
                }
            }
        }
        println!("SSH command handler task finished.");
    });
}

// Task to wait for the child process to exit
fn spawn_wait_handler(
    app_handle: AppHandle,
    state_clone: Arc<Mutex<Option<SshProcessHandle>>>,
    child_arc: Arc<Mutex<Child>>,
) {
    tokio::spawn(async move {
        println!("SSH wait handler task started.");
        let child_id = {
            let mut guard = child_arc.lock().unwrap();
            guard.id()
        };
        println!("Waiting on SSH process ID: {}", child_id);

        let status = {
             let mut child_guard = child_arc.lock().unwrap();
             match child_guard.wait() {
                 Ok(s) => s,
                 Err(e) => {
                     eprintln!("Error waiting for SSH process exit: {}", e);
                     return;
                 }
             }
        };

        println!("SSH process exited with status: {}", status);
        let exit_message = format!("Connection closed. Exit status: {}", status);
        emit_event(&app_handle, "ssh-closed", SshClosedPayload { message: exit_message });

        let mut state_guard = state_clone.lock().unwrap();
        if state_guard.is_some() {
            *state_guard = None;
            println!("SSH handle removed from state by wait handler.");
        }
         println!("SSH wait handler task finished.");
    });
}


// --- Tauri Commands ---

#[tauri::command]
async fn ssh_connect(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    hostname: String,
    port: u16,
    username: String,
    password: Option<String>, // Re-enabled password parameter
) -> Result<(), String> {
    println!(
        "Attempting SSH connection via subprocess to {}@{}:{}",
        username, hostname, port
    );

    // Disconnect any existing session first
    disconnect_ssh_internal(&state).await?;

    // --- Build the command based on OS ---
    let mut command = Command::new(""); // Placeholder

    #[cfg(target_os = "windows")]
    {
        println!("Configuring command for Windows (using Plink).");
        command = Command::new("plink");
        command.arg("-ssh"); // Specify SSH protocol
        command.arg("-P").arg(port.to_string()); // Port
        command.arg("-l").arg(&username); // Username

        if let Some(pass) = password {
            if pass.is_empty() {
                return Err("Password provided but is empty.".to_string());
            }
            println!("Using password authentication with Plink.");
            command.arg("-pw").arg(pass); // Password
        } else {
             println!("Attempting key-based/agent authentication with Plink (no password provided).");
             // Plink can use Pageant or specify key files with -i. Agent auth is often default.
             // Add -tt for pseudo-terminal allocation, similar to ssh.
             command.arg("-tt");
        }
        command.arg(&hostname); // Hostname
    }

    #[cfg(not(target_os = "windows"))]
    {
        println!("Configuring command for non-Windows (using ssh/sshpass).");
        command = if let Some(pass) = password {
            if pass.is_empty() {
                return Err("Password provided but is empty.".to_string());
            }
            println!("Using sshpass for password authentication.");
            let mut cmd = Command::new("sshpass");
            cmd.arg("-p")
               .arg(pass) // Pass the password to sshpass
               .arg("ssh") // Command to run
               .arg(format!("{}@{}", username, hostname))
               .arg("-p")
               .arg(port.to_string())
               // Removed StrictHostKeyChecking options
               .arg("-tt"); // Force pseudo-terminal allocation - IMPORTANT for interactive shells
            cmd
        } else {
            // No password provided, attempt key-based/agent authentication
            println!("Attempting key-based/agent authentication (no password provided).");
            let mut cmd = Command::new("ssh");
            cmd.arg(format!("{}@{}", username, hostname))
               .arg("-p")
               .arg(port.to_string())
               .arg("-tt"); // Force pseudo-terminal allocation
            cmd
        };
    }

    // Configure stdio for the command (either ssh or sshpass)
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());


    // --- Spawn the process ---
    let mut child = command.spawn()
        .map_err(|e| format!("Failed to spawn command: {}. Is sshpass installed if using password?", e))?;

    println!("SSH process spawned with ID: {}", child.id());

    // --- Extract stdio handles ---
    let stdin = child.stdin.take().ok_or("Failed to get stdin handle".to_string())?;
    let stdout = child.stdout.take().ok_or("Failed to get stdout handle".to_string())?;
    let stderr = child.stderr.take().ok_or("Failed to get stderr handle".to_string())?;

    // --- Setup Communication Channels ---
    let (command_tx, command_rx) = mpsc::channel::<SshCommand>(32);

    // --- Wrap handles in Arc<Mutex<>> for sharing ---
    let child_arc = Arc::new(Mutex::new(child));
    let stdin_arc = Arc::new(Mutex::new(stdin));

    // --- Spawn I/O and management tasks ---
    let state_clone = Arc::clone(&state.ssh_handle);
    let handle_clone = app_handle.clone();
    spawn_stdout_reader(handle_clone, stdout); // Updated reader

    let handle_clone = app_handle.clone();
    spawn_stderr_reader(handle_clone, stderr); // Updated reader

    let handle_clone = app_handle.clone();
    let child_clone_cmd = Arc::clone(&child_arc);
    let stdin_clone_cmd = Arc::clone(&stdin_arc);
    let state_clone_cmd = Arc::clone(&state.ssh_handle);
    spawn_command_handler(handle_clone, state_clone_cmd, command_rx, child_clone_cmd, stdin_clone_cmd);

    let handle_clone = app_handle.clone();
    let child_clone_wait = Arc::clone(&child_arc);
    let state_clone_wait = Arc::clone(&state.ssh_handle);
    spawn_wait_handler(handle_clone, state_clone_wait, child_clone_wait);


    // --- Store Handle in State ---
    {
        let mut handle_guard = state.ssh_handle.lock().map_err(|_| "Failed to lock state mutex".to_string())?;
        *handle_guard = Some(SshProcessHandle {
            child: child_arc,
            stdin: stdin_arc,
            command_sender: command_tx,
        });
    }

    println!("SSH connection process setup completed successfully.");
    Ok(())
}

#[tauri::command]
async fn write_to_ssh(state: State<'_, AppState>, data: String) -> Result<(), String> {
    let command_sender = {
        let guard = state.ssh_handle.lock().map_err(|_| "Failed to lock state mutex".to_string())?;
        guard.as_ref().map(|handle| handle.command_sender.clone())
    };

    if let Some(sender) = command_sender {
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
    disconnect_ssh_internal(&state).await
}

// Internal disconnect logic
async fn disconnect_ssh_internal(state: &AppState) -> Result<(), String> {
    println!("Attempting to disconnect SSH process...");

    let command_sender = {
        let mut guard = state.ssh_handle.lock().map_err(|_| "Failed to lock state mutex for disconnect".to_string())?;
        guard.take().map(|handle| handle.command_sender)
    };

    if let Some(sender) = command_sender {
        println!("Found active SSH handle. Sending disconnect command...");
        let _ = sender.send(SshCommand::Disconnect).await;
        println!("Disconnect command sent to command handler.");
    } else {
        println!("No active SSH connection found to disconnect.");
    }

    Ok(())
}

// --- AI Interaction Command ---

#[tauri::command]
async fn ai_write_to_ssh(state: State<'_, AppState>, data: String) -> Result<(), String> {
    println!("AI attempting to write to SSH: {:?}", data); // Log AI writes
    let command_sender = {
        let guard = state.ssh_handle.lock().map_err(|_| "Failed to lock state mutex".to_string())?;
        guard.as_ref().map(|handle| handle.command_sender.clone())
    };

    if let Some(sender) = command_sender {
        sender
            .send(SshCommand::Write(data.into_bytes()))
            .await
            .map_err(|e| format!("AI failed to send write command: {}", e))
    } else {
        Err("AI write failed: Not connected".to_string())
    }
}


// --- Main Application Setup ---
fn main() {
    // Load environment variables from .env file in src-tauri directory
    // It's okay if the file doesn't exist or fails to load.
    dotenvy::dotenv().ok();

    let app_state = AppState::new(); // AppState remains unchanged from original

    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().build())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            ssh_connect,
            write_to_ssh,
            disconnect_ssh,
            gemini_api::send_to_gemini, // Existing command
            ai_write_to_ssh           // <-- Add new AI write command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
