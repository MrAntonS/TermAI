// Example in your Svelte component's script

let host = '';
let port = 22;
let username = '';
let password = ''; // Or keyPath = ''
let output = '';
let error = '';
let isLoading = false;

import { invoke } from '@tauri-apps/api/core';

export async function connectAndRunCommand(command: string) {
  isLoading = true;
  output = '';
  error = '';
  try {
    console.log('Attempting SSH connection...');
    const result = await invoke('ssh_connect_and_run', { // Command name defined in Rust
      hostname: host,
      port: port,
      username: username,
      // Pass either password or key path based on user selection
      password: password || null, // Send null if using key
      // keyPath: keyPath // Send null if using password
      command: command // e.g., 'uptime' or taken from another input
    });
    output = result as string; // Assuming Rust returns the stdout
    console.log('SSH connection successful:', output);
  } catch (err: any) {
    error = err as string; // Error message from Rust
    console.error('SSH connection failed:', error);
  } finally {
    isLoading = false;
  }
}