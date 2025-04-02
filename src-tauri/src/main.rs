#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
  )]
  
  use std::io::{Read, Write};
  use std::net::TcpStream;
  use ssh2::Session;
  use std::collections::HashMap;
  use std::sync::{Mutex, Arc};
  use once_cell::sync::Lazy;
  use std::thread;
  use tauri::{State, Window};
  use tauri::Emitter;
  
  #[derive(Clone, serde::Serialize)]
  struct Payload {
    message: String,
    stream_type: String,  // "stdout", "stderr", or "info"
    connection_id: String,
  }
  
  #[derive(Clone, serde::Serialize, serde::Deserialize)]
  struct SSHConnectionInfo {
    success: bool,
    message: String,
    connection_id: String,
  }
  
  struct ChannelHolder {
    channel: ssh2::Channel,
    active: bool,
  }
  
  struct SSHSession {
    session: Session,
    hostname: String,
    port: u16,
    username: String,
    interactive_channel: Option<Arc<Mutex<ChannelHolder>>>,
  }
  
  struct AppState {
    sessions: Arc<Mutex<HashMap<String, SSHSession>>>,
  }
  
  impl AppState {
    fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }
  }
  
  // Keep original command-based functions
  #[tauri::command]
  fn ssh_connect_and_run(hostname: String, port: u16, username: String, password: Option<String>, command: String) -> Result<String, String> {
    // --- Print detailed information about the connection parameters ---
    println!("=== SSH CONNECTION PARAMETERS ===");
    println!("Hostname: '{}'", hostname);
    println!("Port: {}", port);
    println!("Username: '{}'", username);
    println!("Password provided: {}", password.is_some());
    println!("Command: '{}'", command);
    println!("===============================");
    
    // --- Validate connection parameters ---
    if hostname.trim().is_empty() {
        let err_msg = "Empty hostname provided. Please enter a valid hostname or IP address.".to_string();
        println!("VALIDATION ERROR: {}", err_msg);
        return Err(err_msg);
    }
    
    if username.trim().is_empty() {
        let err_msg = "Empty username provided. Please enter a valid username.".to_string();
        println!("VALIDATION ERROR: {}", err_msg);
        return Err(err_msg);
    }
    
    if command.trim().is_empty() {
        let err_msg = "Empty command provided. Please enter a valid command to execute.".to_string();
        println!("VALIDATION ERROR: {}", err_msg);
        return Err(err_msg);
    }
    
    if port < 1 || port > 65535 {
        let err_msg = format!("Invalid port number: {}. Port must be between 1 and 65535.", port);
        println!("VALIDATION ERROR: {}", err_msg);
        return Err(err_msg);
    }
    
    println!("SSH Connect: Attempting to connect to {}:{} as {}", hostname, port, username);
  
    let tcp = match TcpStream::connect((hostname.as_str(), port)) {
        Ok(stream) => {
            println!("TCP Connection succeeded");
            stream
        },
        Err(e) => {
            // Create more specific error messages for common connection problems
            let err_msg = if e.kind() == std::io::ErrorKind::AddrNotAvailable || 
                          e.to_string().contains("failed to lookup") ||
                          e.to_string().contains("nodename nor servname provided") {
                format!("Failed to resolve hostname '{}': DNS lookup failed. Please check the hostname and your network connection.", hostname)
            } else if e.kind() == std::io::ErrorKind::ConnectionRefused {
                format!("Connection refused to {}:{}. Please check if the SSH service is running and the port is correct.", hostname, port)
            } else if e.kind() == std::io::ErrorKind::TimedOut {
                format!("Connection timed out to {}:{}. Please check your network and firewall settings.", hostname, port)
            } else {
                format!("Failed to connect: {}", e)
            };
            
            println!("ERROR: {}", err_msg);
            println!("ERROR KIND: {:?}", e.kind());
            return Err(err_msg);
        }
    };
    
    println!("SSH Connect: TCP connection established");
  
    let mut sess = Session::new()
        .map_err(|e| {
            let err_msg = format!("Failed to create session: {}", e);
            println!("ERROR: {}", err_msg);
            err_msg
        })?;
    
    println!("SSH Connect: Session created");
    sess.set_tcp_stream(tcp);
    
    println!("SSH Connect: Starting handshake");
    sess.handshake()
        .map_err(|e| {
            let err_msg = format!("SSH Handshake failed: {}", e);
            println!("ERROR: {}", err_msg);
            err_msg
        })?;
    
    println!("SSH Connect: Handshake successful");
  
    // --- Authentication ---
    if let Some(pass) = password {
        println!("SSH Connect: Attempting password authentication");
        sess.userauth_password(&username, &pass)
            .map_err(|e| {
                let err_msg = format!("Authentication failed: {}", e);
                println!("ERROR: {}", err_msg);
                err_msg
            })?;
    } else {
        // TODO: Implement private key authentication here using e.g. userauth_pubkey_file
        let err_msg = "Password or Key authentication required".to_string();
        println!("ERROR: {}", err_msg);
        return Err(err_msg);
    }
  
    if !sess.authenticated() {
        let err_msg = "Authentication failed (method rejected or incomplete)".to_string();
        println!("ERROR: {}", err_msg);
        return Err(err_msg);
    }
    
    println!("SSH Connect: Authentication successful");
  
    // --- Execute Command ---
    println!("SSH Connect: Opening channel");
    let mut channel = sess.channel_session()
        .map_err(|e| {
            let err_msg = format!("Failed to open channel: {}", e);
            println!("ERROR: {}", err_msg);
            err_msg
        })?;
  
    println!("SSH Connect: Executing command: {}", command);
    channel.exec(&command)
        .map_err(|e| {
            let err_msg = format!("Failed to execute command: {}", e);
            println!("ERROR: {}", err_msg);
            err_msg
        })?;
  
    // --- Read Output ---
    println!("SSH Connect: Reading command output");
    let mut output = String::new();
    channel.read_to_string(&mut output)
         .map_err(|e| {
            let err_msg = format!("Failed to read command output: {}", e);
            println!("ERROR: {}", err_msg);
            err_msg
         })?;
  
    // --- Cleanup ---
    println!("SSH Connect: Waiting for channel to close");
    match channel.wait_close() {
        Ok(_) => println!("SSH Connect: Channel closed successfully"),
        Err(e) => println!("WARNING: Error closing channel: {}", e),
    }
    
    println!("SSH Connect: Command executed successfully");
    println!("SSH Connect: Output: {}", output);
  
    Ok(output)
  }
  
  // New functions for interactive shell support
  
  // Start an interactive shell session
  #[tauri::command]
  fn ssh_start_shell(
      state: State<AppState>,
      window: Window,
      hostname: String, 
      port: u16, 
      username: String, 
      password: Option<String>
  ) -> Result<SSHConnectionInfo, String> {
      // --- Validate connection parameters ---
      if hostname.trim().is_empty() {
          return Err("Empty hostname provided. Please enter a valid hostname or IP address.".to_string());
      }
      
      if username.trim().is_empty() {
          return Err("Empty username provided. Please enter a valid username.".to_string());
      }
      
      if port < 1 || port > 65535 {
          return Err(format!("Invalid port number: {}. Port must be between 1 and 65535.", port));
      }
      
      println!("SSH Shell: Attempting to connect to {}:{} as {}", hostname, port, username);
    
      // Establish TCP connection
      let tcp = match TcpStream::connect((hostname.as_str(), port)) {
          Ok(stream) => {
              println!("TCP Connection succeeded");
              stream
          },
          Err(e) => {
              let err_msg = if e.kind() == std::io::ErrorKind::AddrNotAvailable { 
                  format!("Failed to resolve hostname '{}': DNS lookup failed.", hostname)
              } else if e.kind() == std::io::ErrorKind::ConnectionRefused {
                  format!("Connection refused to {}:{}. Please check if SSH service is running.", hostname, port)
              } else if e.kind() == std::io::ErrorKind::TimedOut {
                  format!("Connection timed out to {}:{}. Check your network settings.", hostname, port)
              } else {
                  format!("Failed to connect: {}", e)
              };
              
              return Err(err_msg);
          }
      };
      
      // Create SSH session
      let mut sess = Session::new()
          .map_err(|e| format!("Failed to create session: {}", e))?;
      
      sess.set_tcp_stream(tcp);
      
      sess.handshake()
          .map_err(|e| format!("SSH Handshake failed: {}", e))?;
    
      // Authenticate
      if let Some(pass) = password {
          sess.userauth_password(&username, &pass)
              .map_err(|e| format!("Authentication failed: {}", e))?;
      } else {
          return Err("Password or Key authentication required".to_string());
      }
    
      if !sess.authenticated() {
          return Err("Authentication failed (method rejected or incomplete)".to_string());
      }
      
      println!("SSH Shell: Authentication successful");
      
      // Open a channel for the shell
      let mut channel = sess.channel_session()
          .map_err(|e| format!("Failed to open channel: {}", e))?;
      
      // Request a pseudo-terminal (pty)
      channel.request_pty("xterm", None, None)
          .map_err(|e| format!("Failed to request PTY: {}", e))?;
          
      // Start the shell
      channel.shell()
          .map_err(|e| format!("Failed to start shell: {}", e))?;
      
      // Generate a unique connection ID
      use std::time::{SystemTime, UNIX_EPOCH};
      let timestamp = SystemTime::now()
          .duration_since(UNIX_EPOCH)
          .unwrap_or_default()
          .as_secs();
      let connection_id = format!("shell_{}", timestamp);
      
      // Wrap channel in thread-safe containers
      let channel_holder = Arc::new(Mutex::new(ChannelHolder {
          channel,
          active: true,
      }));
      
      // Clone references for the read thread
      let window_clone = window.clone();
      let connection_id_clone = connection_id.clone();
      let channel_ref = Arc::clone(&channel_holder);
      
      // Start a thread to continuously read from the shell
      thread::spawn(move || {
          let mut buffer = [0u8; 1024];
          
          loop {
              // Check if channel is still active
              {
                  let channel_lock = channel_ref.lock().unwrap();
                  if !channel_lock.active {
                      println!("Shell reader thread shutting down: channel marked as inactive");
                      break;
                  }
              }
              
              // Try to read from channel
              let bytes_read = {
                  let mut channel_lock = channel_ref.lock().unwrap();
                  match channel_lock.channel.read(&mut buffer) {
                      Ok(0) => {
                          // EOF reached
                          println!("Shell EOF reached");
                          channel_lock.active = false;
                          window_clone.emit("ssh_shell_event", Payload {
                              message: "Connection closed by server".to_string(),
                              stream_type: "info".to_string(),
                              connection_id: connection_id_clone.clone(),
                          }).ok();
                          break;
                      },
                      Ok(n) => n,
                      Err(e) => {
                          println!("Error reading from shell: {}", e);
                          channel_lock.active = false;
                          window_clone.emit("ssh_shell_event", Payload {
                              message: format!("Error: {}", e),
                              stream_type: "error".to_string(),
                              connection_id: connection_id_clone.clone(),
                          }).ok();
                          break;
                      }
                  }
              };
              
              // Convert bytes to string and emit to frontend
              match std::str::from_utf8(&buffer[0..bytes_read]) {
                  Ok(s) => {
                      window_clone.emit("ssh_shell_output", Payload {
                          message: s.to_string(),
                          stream_type: "stdout".to_string(),
                          connection_id: connection_id_clone.clone(),
                      }).ok();
                  },
                  Err(e) => {
                      println!("Error converting bytes to string: {}", e);
                      // Send as binary data encoded as hex
                      let hex_str = buffer[0..bytes_read].iter()
                          .map(|b| format!("{:02x}", b))
                          .collect::<String>();
                      window_clone.emit("ssh_shell_output", Payload {
                          message: format!("[Binary data: {}]", hex_str),
                          stream_type: "binary".to_string(),
                          connection_id: connection_id_clone.clone(),
                      }).ok();
                  }
              }
              
              // Small sleep to prevent CPU hogging
              std::thread::sleep(std::time::Duration::from_millis(10));
          }
          
          println!("Shell reader thread for {} terminated", connection_id_clone);
      });
      
      // Store the session
      {
          let mut sessions = state.sessions.lock().unwrap();
          sessions.insert(connection_id.clone(), SSHSession {
              session: sess,
              hostname: hostname.clone(),
              port,
              username: username.clone(),
              interactive_channel: Some(channel_holder),
          });
          println!("Stored interactive shell session with ID: {}", connection_id);
      }
      
      // Return success info
      Ok(SSHConnectionInfo {
          success: true,
          message: format!("Successfully connected to {}:{} as {} (interactive shell)", hostname, port, username),
          connection_id
      })
  }
  
  // Send input to the shell
  #[tauri::command]
  fn ssh_shell_send(
      state: State<AppState>,
      connection_id: String, 
      input: String
  ) -> Result<bool, String> {
      println!("Sending input to shell: {}", connection_id);
      
      // Get the session
      let sessions = state.sessions.lock().unwrap();
      let session = match sessions.get(&connection_id) {
          Some(session) => session,
          None => return Err(format!("Connection ID not found: {}", connection_id)),
      };
      
      // Get the channel
      let channel_holder = match &session.interactive_channel {
          Some(ch) => ch,
          None => return Err("No interactive shell found for this connection".to_string()),
      };
      
      // Write to the channel
      {
          let mut channel_lock = channel_holder.lock().unwrap();
          if !channel_lock.active {
              return Err("Shell is no longer active".to_string());
          }
          
          // Write input to channel
          match channel_lock.channel.write_all(input.as_bytes()) {
              Ok(_) => (),
              Err(e) => return Err(format!("Failed to send input: {}", e)),
          }
      }
      
      Ok(true)
  }
  
  // Resize the terminal
  #[tauri::command]
  fn ssh_resize_pty(
      state: State<AppState>,
      connection_id: String, 
      width: u32, 
      height: u32
  ) -> Result<bool, String> {
      println!("Resizing PTY for shell: {}, {}x{}", connection_id, width, height);
      
      // Get the session
      let sessions = state.sessions.lock().unwrap();
      let session = match sessions.get(&connection_id) {
          Some(session) => session,
          None => return Err(format!("Connection ID not found: {}", connection_id)),
      };
      
      // Get the channel
      let channel_holder = match &session.interactive_channel {
          Some(ch) => ch,
          None => return Err("No interactive shell found for this connection".to_string()),
      };
      
      // Resize the PTY
      {
          let mut channel_lock = channel_holder.lock().unwrap();
          if !channel_lock.active {
              return Err("Shell is no longer active".to_string());
          }
          
          // Request PTY size change
          match channel_lock.channel.request_pty_size(width as u32, height as u32, None, None) {
              Ok(_) => (),
              Err(e) => return Err(format!("Failed to resize PTY: {}", e)),
          }
      }
      
      Ok(true)
  }
  
  // Close the shell
  #[tauri::command]
  fn ssh_close_shell(
      state: State<AppState>,
      connection_id: String
  ) -> Result<bool, String> {
      println!("Closing shell: {}", connection_id);
      
      // Mark channel as inactive and remove session
      {
          let mut sessions = state.sessions.lock().unwrap();
          if let Some(session) = sessions.get(&connection_id) {
              if let Some(channel_holder) = &session.interactive_channel {
                  let mut channel_lock = channel_holder.lock().unwrap();
                  channel_lock.active = false;
                  
                  // Try to send exit command
                  let _ = channel_lock.channel.write_all(b"exit\n");
              }
          }
          
          // Remove the session
          if sessions.remove(&connection_id).is_none() {
              return Err(format!("Connection ID not found: {}", connection_id));
          }
      }
      
      Ok(true)
  }
  
  // Keep the existing disconnect function
  #[tauri::command]
  fn ssh_disconnect(state: State<AppState>, connection_id: String) -> Result<bool, String> {
      println!("Disconnecting session: {}", connection_id);
      
      // Remove the session from our map
      let mut sessions = state.sessions.lock().unwrap();
      match sessions.remove(&connection_id) {
          Some(_) => {
              println!("Session disconnected: {}", connection_id);
              Ok(true)
          },
          None => {
              let err_msg = format!("Connection ID not found: {}", connection_id);
              println!("ERROR: {}", err_msg);
              Err(err_msg)
          }
      }
  }
  
  fn main() {
      println!("Starting application");
      
      let app_state = AppState::new();
      
      match tauri::Builder::default()
          .manage(app_state)
          .invoke_handler(tauri::generate_handler![
              ssh_connect_and_run,      // Original command
              ssh_start_shell,          // Start an interactive shell
              ssh_shell_send,           // Send input to shell
              ssh_resize_pty,           // Resize the terminal
              ssh_close_shell,          // Close the shell
              ssh_disconnect            // Original disconnect command
          ])
          .run(tauri::generate_context!()) {
              Ok(_) => println!("Application exited normally"),
              Err(e) => {
                  eprintln!("ERROR: Application error: {}", e);
                  eprintln!("ERROR DETAILS: {:?}", e);
              }
          }
  }