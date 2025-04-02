<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { browser } from '$app/environment';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import type { EventCallback, UnlistenFn } from '@tauri-apps/api/event';

  // Type Imports
  import type { Terminal as XtermTerminal, IDisposable } from '@xterm/xterm';
  import type { FitAddon as XtermFitAddon } from '@xterm/addon-fit';

  // --- Component State ---
  let terminalContainer: HTMLDivElement;
  let term: XtermTerminal | null = null;
  let fitAddon: XtermFitAddon | null = null;
  let isConnected = false; // Track connection status
  let statusMessage = 'Initializing...';

  // --- Event Listener Cleanup ---
  let unlistenOutput: UnlistenFn | null = null;
  let unlistenError: UnlistenFn | null = null;
  let unlistenClosed: UnlistenFn | null = null;
  let xtermDataListener: IDisposable | null = null; // For xterm's onData

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
      statusMessage = 'Terminal Ready. Not Connected.';
      term.writeln('\x1b[33mWelcome! Please connect to an SSH server.\x1b[0m'); // Yellow text

      // --- Fit Terminal ---
      setTimeout(() => fitAddon?.fit(), 50); // Adjust delay if needed

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
        // Optionally update connection status on certain errors
      };
      const handleClosed: EventCallback<SshClosedPayload> = (event) => {
        console.log('ssh-closed received:', event.payload);
        isConnected = false;
        statusMessage = `Disconnected: ${event.payload.message}`;
        term?.writeln(`\r\n\x1b[33mConnection closed: ${event.payload.message}\x1b[0m`); // Yellow message
      };

      unlistenOutput = await listen('ssh-output', handleOutput);
      unlistenError = await listen('ssh-error', handleError);
      unlistenClosed = await listen('ssh-closed', handleClosed);

      console.log("Terminal and event listeners initialized.");

      // Example: Automatically try to connect on mount (remove/modify as needed)
      // connect('test.rebex.net', 22, 'demo', 'password');

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
    if (isConnected) {
      try {
        console.log("Attempting to disconnect SSH via Tauri command...");
        await invoke('disconnect_ssh');
        console.log("disconnect_ssh command invoked.");
        isConnected = false; // Assume disconnect succeeded for cleanup
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
    console.log("Terminal component destroyed.");
  });

  // --- Exposed Methods / Connection Handling ---
  // You'll likely call these from a parent component or UI elements
  export async function connect(host: string, port: number, user: string, pass: string | null) {
    if (isConnected) {
      term?.writeln('\r\n\x1b[33mAlready connected. Disconnect first.\x1b[0m');
      return;
    }
    if (!term) {
       console.error("Terminal not initialized, cannot connect.");
       statusMessage = "Error: Terminal not ready";
       return;
    }

    term.reset(); // Clear the terminal screen
    term.writeln(`\x1b[36mConnecting to ${user}@${host}:${port}...\x1b[0m`); // Cyan message
    statusMessage = `Connecting to ${host}...`;

    try {
      await invoke('connect_ssh', {
        hostname: host,
        port: port,
        username: user,
        password: pass // Pass null if no password (key auth not implemented in backend yet)
      });
      isConnected = true;
      statusMessage = `Connected to ${host}`;
      term.writeln(`\x1b[32mConnection successful!\x1b[0m`); // Green message
      term.focus(); // Focus the terminal for input
    } catch (error) {
      console.error("SSH Connection failed:", error);
      isConnected = false;
      statusMessage = `Connection Failed: ${error}`;
      term.writeln(`\r\n\x1b[31mConnection Failed: ${error}\x1b[0m`); // Red error
    }
  }

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
      // Force state update if command fails but we want to reflect attempt
      isConnected = false;
    }
  }

  // Example function to resize terminal (call when container resizes)
  export function fitTerminal() {
    try {
        fitAddon?.fit();
        // Optional: Inform backend about resize if needed (requires backend command)
        // const dims = term?.proposeDimensions();
        // if (dims && isConnected) {
        //   invoke('resize_pty', { cols: dims.cols, rows: dims.rows });
        // }
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
    Status: {statusMessage} {isConnected ? '🟢' : '🔴'}
  </div>
  <!-- Container where xterm will attach -->
  <div bind:this={terminalContainer} class="terminal-container" tabindex="0"></div>
</div>
