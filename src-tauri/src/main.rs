// Example in src-tauri/src/main.rs (using ssh2 crate - add `ssh2 = "0.9"` to Cargo.toml)
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
  )]
  
  use std::io::Read; // Import the Read trait specifically
  use std::net::TcpStream;
  use ssh2::Session;
  
  #[derive(Clone, serde::Serialize)]
  struct Payload {
    message: String,
  }
  
  // Define the command that the frontend will call
  #[tauri::command]
  fn ssh_connect_and_run(hostname: String, port: u16, username: String, password: Option<String>, command: String) -> Result<String, String> {
    // --- Basic Example using password auth ---
    // TODO: Add private key auth, proper error handling, known_hosts handling etc.
  
    let tcp = TcpStream::connect((hostname.as_str(), port))
        .map_err(|e| format!("Failed to connect: {}", e))?;
  
    let mut sess = Session::new()
        .map_err(|e| format!("Failed to create session: {}", e))?;
    sess.set_tcp_stream(tcp);
    sess.handshake()
        .map_err(|e| format!("SSH Handshake failed: {}", e))?;
  
    // --- Authentication ---
    if let Some(pass) = password {
         sess.userauth_password(&username, &pass)
            .map_err(|e| format!("Authentication failed: {}", e))?;
    } else {
        // TODO: Implement private key authentication here using e.g. userauth_pubkey_file
         return Err("Password or Key authentication required".to_string());
    }
  
    if !sess.authenticated() {
        return Err("Authentication failed (method rejected or incomplete)".to_string());
    }
  
    // --- Execute Command ---
    let mut channel = sess.channel_session()
        .map_err(|e| format!("Failed to open channel: {}", e))?;
  
    channel.exec(&command)
        .map_err(|e| format!("Failed to execute command: {}", e))?;
  
    // --- Read Output ---
    let mut output = String::new();
    // Now that `std::io::Read` is in scope, this method is available
    channel.read_to_string(&mut output)
         .map_err(|e| format!("Failed to read command output: {}", e))?;
  
    // --- Cleanup ---
    channel.wait_close().ok(); // Ignore errors on close for simplicity
    // Session will be closed when `sess` goes out of scope
  
    Ok(output) // Return stdout
    // TODO: Handle stderr separately if needed
  }
  
  
  fn main() -> Result<(), String> {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            ssh_connect_and_run // Register the command
        ])
        .run(tauri::generate_context!())
        .map_err(|e| format!("Tauri application error: {}", e))?; // Handle Tauri run errors
  
    Ok(()) // Return Ok if the Tauri app ran and exited without a launch error
    // Remove the explicit Err(...) line that was here previously
  }