import { invoke } from '@tauri-apps/api/core';
import { writable, get } from 'svelte/store';

// Types
export type ConnectionDetails = {
  hostname: string;
  port: number;
  username: string;
  authMethod: 'password' | 'key';
  password?: string;
  privateKeyPath?: string;
};

export type ActiveConnection = {
  id: string;
  name: string;
  type: string;
  details: ConnectionDetails;
  connectionId: string;
  isActive: boolean;
};

// Create a store for managing active connections
export const activeConnections = writable<ActiveConnection[]>([]);
export const currentConnectionId = writable<string | null>(null);

// Function to add a new connection to the store
export function addConnection(connection: ActiveConnection) {
  activeConnections.update(connections => [connection, ...connections]);
  // If this is the first connection, set it as the current connection
  if (get(activeConnections).length === 1) {
    currentConnectionId.set(connection.id);
  }
}

// Function to run a command on the current connection
export async function runCommand(command: string): Promise<string> {
  const current = get(currentConnectionId);
  if (!current) {
    return "No active connection";
  }
  
  // Find the connection details
  const connection = get(activeConnections).find(c => c.id === current);
  if (!connection) {
    return "Connection not found";
  }
  
  try {
    // Execute command via Rust backend
    const result = await invoke('ssh_run_command', {
      connectionId: connection.connectionId,
      command
    });
    
    return result as string;
  } catch (error) {
    console.error('Error running command:', error);
    return `Error: ${error}`;
  }
}

// Function to set the current connection
export function setCurrentConnection(id: string) {
  currentConnectionId.set(id);
}

// Function to disconnect from a connection
export async function disconnect(id: string): Promise<boolean> {
  const connection = get(activeConnections).find(c => c.id === id);
  if (!connection) {
    return false;
  }
  
  try {
    // Call Rust backend to close the connection
    await invoke('ssh_disconnect', {
      connectionId: connection.connectionId
    });
    
    // Remove from store
    activeConnections.update(connections => connections.filter(c => c.id !== id));
    
    // If this was the current connection, set the first available connection as current
    if (get(currentConnectionId) === id) {
      const connections = get(activeConnections);
      if (connections.length > 0) {
        currentConnectionId.set(connections[0].id);
      } else {
        currentConnectionId.set(null);
      }
    }
    
    return true;
  } catch (error) {
    console.error('Error disconnecting:', error);
    return false;
  }
}