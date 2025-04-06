<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { browser } from '$app/environment';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import type { EventCallback, UnlistenFn } from '@tauri-apps/api/event';
  import { currentConnectionId, activeConnections } from '../Connection Tab/ConnectionStore'; // Import the store

  // Type Imports
  import type { Terminal as XtermTerminal, IDisposable } from '@xterm/xterm';
  import type { FitAddon as XtermFitAddon } from '@xterm/addon-fit';

  // --- Component State ---
  let terminalContainer: HTMLDivElement;
  let term: XtermTerminal | null = null;
  let fitAddon: XtermFitAddon | null = null;
  // Remove local isConnected state, derive from store
  // let isConnected = false;
  let statusMessage = $state('Initializing...'); // Use $state for reactivity
 
  // Derive connection status and details from the store
  // Find the connection object from the array using the current ID
  let currentConnection = $derived($activeConnections.find(conn => conn.id === $currentConnectionId));
  let isConnected = $derived(!!currentConnection);
  // --- Event Listener Cleanup ---
  let unlistenOutput: UnlistenFn | null = null;
  let unlistenError: UnlistenFn | null = null;
  let unlistenClosed: UnlistenFn | null = null;
  let xtermDataListener: IDisposable | null = null; // For xterm's onData
  let resizeObserver: ResizeObserver | null = null;
 
  // --- Payload Types (match Rust structs) ---
  interface SshOutputPayload {
    data: string;
  }
  interface SshErrorPayload {
    message: string;
  }
   interface SshClosedPayload {
     message: string;
   }

  // --- Lifecycle ---
  onMount(async () => {
    if (!browser) {
      console.log("Skipping xterm initialization during SSR.");
      statusMessage = 'SSR Mode';
      return;
    }
    if (!terminalContainer) {
      console.error("Terminal container element not found on mount.");
      statusMessage = 'Error: Container not found';
      return;
    }

    try {
      statusMessage = 'Loading xterm...';
      // --- Dynamic Imports ---
      const Xterm = await import('@xterm/xterm');
      const Terminal = Xterm.Terminal;
      const FitAddonPkg = await import('@xterm/addon-fit');
      const FitAddon = FitAddonPkg.FitAddon;

      // --- Initialize Addon & Terminal ---
      fitAddon = new FitAddon();
      term = new Terminal({
        cursorBlink: true,
        fontFamily: 'monospace',
        fontSize: 14,
        theme: {
          background: '#1e1e1e',
          foreground: '#66BB6A', // Green foreground
          cursor: '#ffffff',
          selectionBackground: '#555555'
        },
        // Consider adding convertEol: true if needed
      });

      // --- Load Addons & Open ---
      term.loadAddon(fitAddon);
      term.open(terminalContainer);
      // Update status based on derived state initially
      statusMessage = isConnected ? `Connected to ${currentConnection?.name}` : 'Terminal Ready. Not Connected.';
      if (!isConnected) {
        term.writeln('\x1b[33mWelcome! Please select or establish an SSH connection.\x1b[0m'); // Yellow text
      } else {
        term.writeln(`\x1b[32mTerminal attached to active connection: ${currentConnection?.name}\x1b[0m`); // Green text
      }
      // --- Fit Terminal (Initial) ---
      fitTerminal(); // Call immediately after open

      // --- Setup Input Handling (Send data to backend) ---
      xtermDataListener = term.onData(async (data) => {
        if (isConnected) {
          try {
            await invoke('write_to_ssh', { data });
          } catch (error) {
            console.error("Failed to send data to SSH:", error);
            term?.writeln(`\r\n\x1b[31mError sending data: ${error}\x1b[0m`); // Red error
          }
        } else {
          term?.writeln('\r\n\x1b[31mNot connected. Cannot send data.\x1b[0m'); // Red error
        }
      });

      // --- Setup Event Listeners (Receive data from backend) ---
      const handleOutput: EventCallback<SshOutputPayload> = (event) => {
        // console.log('ssh-output received:', event.payload);
        term?.write(event.payload.data); // Write data directly
      };
      const handleError: EventCallback<SshErrorPayload> = (event) => {
        console.error('ssh-error received:', event.payload);
        term?.writeln(`\r\n\x1b[31mSSH Error: ${event.payload.message}\x1b[0m`); // Red error

      };
      const handleClosed: EventCallback<SshClosedPayload> = (event) => {
        console.log('ssh-closed received:', event.payload);
        statusMessage = `Disconnected`;
        term?.writeln(`\r\n\x1b[33mConnection closed: ${event.payload.message}\x1b[0m`);
      };

      unlistenOutput = await listen('ssh-output', handleOutput);
      unlistenError = await listen('ssh-error', handleError);
      unlistenClosed = await listen('ssh-closed', handleClosed);
console.log("Terminal and event listeners initialized.");

// --- Setup Resize Observer ---
if (window.ResizeObserver) {
    resizeObserver = new ResizeObserver(() => {
        // Debounce or throttle this if performance becomes an issue
        fitTerminal();
    });
    resizeObserver.observe(terminalContainer);
    console.log("ResizeObserver attached to terminal container.");
} else {
    console.warn("ResizeObserver not supported. Terminal resizing may not work automatically.");
    // Consider adding a fallback, like a manual resize button or window resize listener
}

// No longer need to auto-connect here
      // No longer need to auto-connect here
    } catch (error) {
      console.error("Failed to load or initialize xterm:", error);
      statusMessage = `Error: ${error}`;
      if (terminalContainer) {
        terminalContainer.innerText = `Error loading terminal: ${error}`;
      }
    }
  });

  onDestroy(async () => {
    console.log("Destroying Terminal component...");

    // 1. Disconnect SSH backend if connected
    // Use the derived isConnected state
    if (isConnected && currentConnection?.connectionId) { // Check if we have an active connection ID
      try {
        console.log(`Attempting to disconnect SSH connection ${currentConnection.connectionId} via Tauri command...`);
        // Pass the specific connection ID if your backend disconnect needs it
        // If disconnect_ssh disconnects the *single* backend connection, no ID is needed.
        await invoke('disconnect_ssh');
        console.log("disconnect_ssh command invoked.");
        // isConnected will update reactively. No need to set it manually.
      } catch (error) {
        console.error("Error invoking disconnect_ssh:", error);
        // Continue cleanup even if disconnect fails
      }
    }

    // 2. Unsubscribe from Tauri events
    unlistenOutput?.();
    unlistenError?.();
    unlistenClosed?.();
    console.log("Tauri event listeners detached.");

    // 3. Dispose xterm listener
    xtermDataListener?.dispose();
    console.log("Xterm data listener disposed.");

    // 4. Disconnect ResizeObserver
    resizeObserver?.disconnect();
    console.log("ResizeObserver disconnected.");

    // 4. Dispose xterm instance
    if (term) {
      term.dispose();
      console.log("Xterm instance disposed.");
    }

    // 5. Nullify references
    term = null;
    fitAddon = null;
    unlistenOutput = null;
    unlistenError = null;
    unlistenClosed = null;
    xtermDataListener = null;
    resizeObserver = null;
    console.log("Terminal component destroyed.");
  });

  // --- Connection Handling (Reflecting Store State) ---
  // Remove the connect function as connection is managed externally via QuickConnectModal/ConnectionStore

  // Use $effect for side effects based on reactive state changes
  $effect(() => {
    // This effect runs when isConnected or currentConnection changes,
    // and also after the component mounts if browser and term are ready.
    if (browser && term) {
      const connected = isConnected; // Capture derived value for consistent check within effect
      const connection = currentConnection; // Capture derived value

      if (connected && connection) {
        // State when connected
        const newStatus = `Connected to ${connection.name}`;
        // Update status message and terminal output if the status actually changed
        if (statusMessage !== newStatus) {
            statusMessage = newStatus;
            // Optional: Clear terminal on new connection? Or keep history?
            // term.reset();
            term.writeln(`\r\n\x1b[32mAttached to active connection: ${connection.name}\x1b[0m`);
            term.focus();
        }
      } else {
        // State when not connected
        const newStatus = 'Not Connected';
        // Update status message and terminal output if the status actually changed
        if (statusMessage !== newStatus) {
            statusMessage = newStatus;
            // Optional: Clear terminal on disconnect?
            // term.reset();
            term.writeln('\r\n\x1b[33mNo active SSH connection.\x1b[0m');
        }
      }
    }
  });

  export async function disconnect() {
    if (!isConnected) {
      term?.writeln('\r\n\x1b[33mNot currently connected.\x1b[0m');
      return;
    }
    if (!term) {
       console.error("Terminal not initialized, cannot disconnect.");
       return;
    }

    term.writeln('\r\n\x1b[36mDisconnecting...\x1b[0m'); // Cyan message
    statusMessage = 'Disconnecting...';

    try {
      await invoke('disconnect_ssh');
      // The handleClosed listener will set isConnected = false and update status
      // isConnected = false; // Avoid race condition, let event handle it
      // statusMessage = 'Disconnected';
      term.writeln('\r\n\x1b[33mDisconnect command sent.\x1b[0m');
    } catch (error) {
      console.error("SSH Disconnection failed:", error);
      statusMessage = `Disconnect Failed: ${error}`;
      term?.writeln(`\r\n\x1b[31mDisconnection Failed: ${error}\x1b[0m`); // Red error
      // isConnected updates reactively. If disconnect fails, the store state won't change,
      // and isConnected will remain true, which might be the desired behavior (reflecting reality).
      // If you want to force isConnected to false even on failed disconnect, you'd need to update the store.
    }
  }

  // Example function to resize terminal (call when container resizes)
  export function fitTerminal() {
    try {
        fitAddon?.fit();
        // Inform backend about resize
        const dims = fitAddon?.proposeDimensions();
        // Only send resize if connected and dimensions are valid
        if (dims && isConnected && currentConnection?.connectionId) {
            invoke('resize_pty', { cols: dims.cols, rows: dims.rows }).catch(e => {
                console.error("Failed to invoke resize_pty:", e);
                // Handle error - maybe the command doesn't exist or failed
            });
        }
    } catch (e) {
        console.error("Error fitting terminal:", e);
    }
  }

</script>

<style>
  .terminal-wrapper {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    background-color: #1e1e1e;
    border: 1px solid #444;
    border-radius: 4px;
    overflow: hidden; /* Prevent wrapper scrollbars */
  }
  .terminal-status-bar {
    background-color: #333;
    color: #ccc;
    padding: 2px 8px;
    font-size: 0.8em;
    flex-shrink: 0; /* Prevent shrinking */
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .terminal-container {
    flex-grow: 1; /* Allow terminal to fill remaining space */
    width: 100%;
    height: 100%; /* Fill the flex item */
    overflow: hidden; /* Let xterm handle scrolling */
    padding: 5px;
    box-sizing: border-box;
    position: relative;
  }

  /* Global styles for xterm */
  :global(.terminal.xterm) {
      height: 100% !important;
      width: 100% !important;
      padding: 0;
      box-sizing: border-box;
      outline: none;
  }
  :global(.xterm-viewport) {
      width: 100% !important;
      overflow-y: auto !important; /* Ensure xterm's viewport scrolls */
      box-sizing: border-box;
  }
   :global(.xterm-screen) {
       height: 100% !important;
       width: 100% !important;
       box-sizing: border-box;
   }
</style>

<div class="terminal-wrapper">
  <div class="terminal-status-bar">
    Status: {isConnected ? "Connected" : "Not Connected"} {isConnected ? `🟢 (${currentConnection?.name ?? 'Unknown'})` : '🔴'}
  </div>
  <!-- Container where xterm will attach -->
  <div bind:this={terminalContainer} class="terminal-container" tabindex="0"></div>
</div>
