// Example in src-tauri/src/main.rs (using ssh2 crate - add `ssh2 = "0.9"` to Cargo.toml)
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
  )]
  
  use std::io::Read; // Import the Read trait specifically
  use std::net::TcpStream;
  use ssh2::Session;
  use std::collections::HashMap;
  use std::sync::Mutex;
  use once_cell::sync::Lazy;
  
  #[derive(Clone, serde::Serialize)]
  struct Payload {
    message: String,
    // Added debug field to see if it causes an error
    debug_info: Option<String>,
  }
  
  // Structure to represent an established SSH session
  #[derive(Clone, serde::Serialize, serde::Deserialize)]
  struct SSHConnectionInfo {
    success: bool,
    message: String,
    connection_id: String,
  }
  
  // Global storage for active SSH sessions
  struct SSHSession {
    session: Session,
    hostname: String,
    port: u16,
    username: String,
  }
  
  // Static storage for SSH sessions with thread-safe access
  static SSH_SESSIONS: Lazy<Mutex<HashMap<String, SSHSession>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
  });
  
  // Define the command that the frontend will call
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
    
    // --- Basic Example using password auth ---
    // TODO: Add private key auth, proper error handling, known_hosts handling etc.
    println!("SSH Connect: Attempting to connect to {}:{} as {}", hostname, port, username);
  
    // Attempt to connect with better error handling
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
    // Now that `std::io::Read` is in scope, this method is available
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
    // Session will be closed when `sess` goes out of scope
    
    println!("SSH Connect: Command executed successfully");
    println!("SSH Connect: Output: {}", output);
  
    Ok(output) // Return stdout
    // TODO: Handle stderr separately if needed
  }
  
  
  // Connect-only function that doesn't stream output to the terminal
  #[tauri::command]
  fn ssh_connect_only(hostname: String, port: u16, username: String, password: Option<String>) -> Result<SSHConnectionInfo, String> {
    // --- Print detailed information about the connection parameters ---
    println!("=== SSH CONNECTION ONLY PARAMETERS ===");
    println!("Hostname: '{}'", hostname);
    println!("Port: {}", port);
    println!("Username: '{}'", username);
    println!("Password provided: {}", password.is_some());
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
    
    if port < 1 || port > 65535 {
        let err_msg = format!("Invalid port number: {}. Port must be between 1 and 65535.", port);
        println!("VALIDATION ERROR: {}", err_msg);
        return Err(err_msg);
    }
    
    // --- Attempt to connect with better error handling
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
    
    // Generate a unique connection ID
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let connection_id = format!("conn_{}", timestamp);
    
    // Store the session in our global map
    {
        let mut sessions = SSH_SESSIONS.lock().unwrap();
        sessions.insert(connection_id.clone(), SSHSession {
            session: sess,
            hostname: hostname.clone(),
            port,
            username: username.clone(),
        });
        println!("Stored session with ID: {}", connection_id);
    }
    
    // Return success info
    Ok(SSHConnectionInfo {
        success: true,
        message: format!("Successfully connected to {}:{} as {}", hostname, port, username),
        connection_id
    })
  }
  
  // Run a command on an established connection
  #[tauri::command]
  fn ssh_run_command(connection_id: String, command: String) -> Result<String, String> {
    println!("Running command '{}' on connection {}", command, connection_id);
    
    // Get the session from our global map
    let mut sessions = SSH_SESSIONS.lock().unwrap();
    let session = match sessions.get_mut(&connection_id) {
        Some(session) => session,
        None => {
            let err_msg = format!("Connection ID not found: {}", connection_id);
            println!("ERROR: {}", err_msg);
            return Err(err_msg);
        }
    };
    
    // Open a channel
    let mut channel = session.session.channel_session()
        .map_err(|e| {
            let err_msg = format!("Failed to open channel: {}", e);
            println!("ERROR: {}", err_msg);
            err_msg
        })?;
    
    // Execute the command
    channel.exec(&command)
        .map_err(|e| {
            let err_msg = format!("Failed to execute command: {}", e);
            println!("ERROR: {}", err_msg);
            err_msg
        })?;
    
    // Read the output
    let mut output = String::new();
    channel.read_to_string(&mut output)
        .map_err(|e| {
            let err_msg = format!("Failed to read command output: {}", e);
            println!("ERROR: {}", err_msg);
            err_msg
        })?;
    
    // Wait for the channel to close
    match channel.wait_close() {
        Ok(_) => println!("Channel closed successfully"),
        Err(e) => println!("WARNING: Error closing channel: {}", e),
    }
    
    Ok(output)
  }
  
  // Disconnect from a session
  #[tauri::command]
  fn ssh_disconnect(connection_id: String) -> Result<bool, String> {
    println!("Disconnecting session: {}", connection_id);
    
    // Remove the session from our global map
    let mut sessions = SSH_SESSIONS.lock().unwrap();
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
    
    match tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            ssh_connect_and_run,     // Register the command for connecting and running
            ssh_connect_only,        // Register the new connect-only command
            ssh_run_command,         // Register the command for running commands on existing connections
            ssh_disconnect           // Register the command for disconnecting sessions
        ])
        .run(tauri::generate_context!()) {
            Ok(_) => println!("Application exited normally"),
            Err(e) => {
                eprintln!("ERROR: Application error: {}", e);
                eprintln!("ERROR DETAILS: {:?}", e);
            }
        }
  }