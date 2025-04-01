// SSH Connection Agent with proper parameter passing

import { invoke } from '@tauri-apps/api/core';

// Types for the connection details
type ConnectionDetails = {
  hostname: string;
  port: number;
  username: string;
  authMethod: 'password' | 'key';
  password?: string;
  privateKeyPath?: string;
};

// Type for connection info returned by connect-only operation
type SSHConnectionInfo = {
  success: boolean;
  message: string;
  connection_id: string;
};

// State for tracking connections
let output = '';
let error = '';
let isLoading = false;

// Function to only establish a connection without streaming output
export async function connectOnly(connectionDetails?: ConnectionDetails): Promise<SSHConnectionInfo> {
  isLoading = true;
  error = '';
  
  // Make sure we have connection details
  if (!connectionDetails) {
    console.error('No connection details provided');
    error = 'Connection details missing';
    isLoading = false;
    throw new Error('Connection details missing');
  }
  
  const { hostname, port, username, password, authMethod } = connectionDetails;
  
  // Log what we're attempting to connect with
  console.log('Attempting SSH connection (connect-only) to:', hostname, port, username);
  
  try {
    console.log('Attempting SSH connection (connect-only)...');
    const result = await invoke('ssh_connect_only', { // Command name defined in Rust
      hostname: hostname,
      port: port,
      username: username,
      // Pass either password or null depending on auth method
      password: authMethod === 'password' ? password : null,
    });
    
    const connectionInfo = result as SSHConnectionInfo;
    console.log('SSH connection successful:', connectionInfo);
    return connectionInfo;
  } catch (err: any) {
    error = err as string; // Error message from Rust
    console.error('SSH connection failed:', error);
    throw error; // Re-throw to let caller handle it
  } finally {
    isLoading = false;
  }
}

export async function connectAndRunCommand(command: string, connectionDetails?: ConnectionDetails) {
  isLoading = true;
  output = '';
  error = '';
  
  // Make sure we have connection details
  if (!connectionDetails) {
    console.error('No connection details provided');
    error = 'Connection details missing';
    isLoading = false;
    return 'Connection details missing';
  }
  
  const { hostname, port, username, password, authMethod } = connectionDetails;
  
  // Log what we're attempting to connect with
  console.log('Attempting SSH connection to:', hostname, port, username);
  
  try {
    console.log('Attempting SSH connection...');
    const result = await invoke('ssh_connect_and_run', { // Command name defined in Rust
      hostname: hostname,
      port: port,
      username: username,
      // Pass either password or null depending on auth method
      password: authMethod === 'password' ? password : null,
      command: command || 'ls' // Default command if none provided
    });
    
    output = result as string; // Assuming Rust returns the stdout
    console.log('SSH connection successful:', output);
    return output;
  } catch (err: any) {
    error = err as string; // Error message from Rust
    console.error('SSH connection failed:', error);
    throw error; // Re-throw to let caller handle it
  } finally {
    isLoading = false;
  }
}