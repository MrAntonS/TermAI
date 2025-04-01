<script lang="ts">
  // Import onMount and onDestroy from svelte
  import { onMount, onDestroy } from 'svelte';
  // Import browser check from sveltekit environment
  import { browser } from '$app/environment';

  // Type Imports (these are safe for SSR as they are removed during compilation)
  import type { Terminal as XtermTerminal } from '@xterm/xterm'; // Use 'type' import
  import type { FitAddon as XtermFitAddon } from '@xterm/addon-fit'; // Use 'type' import

  let terminalContainer: HTMLDivElement; // Reference to the container div
  let term: XtermTerminal | null = null; // The xterm instance, initially null
  let fitAddon: XtermFitAddon | null = null; // The fit addon instance, initially null
  let resizeObserver: ResizeObserver | null = null; // Resize observer instance

  // Use onMount, which only runs in the browser
  onMount(async () => {
    // Double-check we are in the browser before proceeding
    if (browser && terminalContainer) {
      try {
        // --- Dynamic Imports (Client-Side Only) ---
        // Dynamically import xterm main library
        const Xterm = await import('@xterm/xterm');
        const Terminal = Xterm.Terminal; // Get Terminal class from default export

        // Dynamically import the fit addon
        const FitAddonPkg = await import('@xterm/addon-fit');
        const FitAddon = FitAddonPkg.FitAddon; // Get FitAddon class from default export

        // --- Initialize Addon ---
        // Initialize FitAddon only after dynamic import
        fitAddon = new FitAddon();

        // --- Initialize Terminal ---
        // Initialize Terminal only after dynamic import
        term = new Terminal({
          cursorBlink: true,
          fontFamily: 'monospace',
          fontSize: 14,
          theme: {
            background: '#1e1e1e',
            foreground: '#66BB6A',
            cursor: '#ffffff',
            selectionBackground: '#555555'
          },
          // scrollback: 1000, // Example option
        });

        // --- Load Addons ---
        term.loadAddon(fitAddon);

        // --- Open Terminal ---
        term.open(terminalContainer);

        // --- Fit Terminal Initially ---
        setTimeout(() => {
          try {
            fitAddon?.fit(); // Use optional chaining ?. just in case
          } catch (e) {
            console.error("Error during initial fit:", e);
          }
        }, 50);

        // --- Write Initial Content ---
        term.writeln('Welcome to the terminal!');
        term.write('$ ');
        
                // --- Handle User Input ---
                term?.onData(data => {
                  if (data === '\x08' || data === '\x7F') { // Backspace or Delete
                    // Handle backspace/delete
                    term?.write('\x08 \x08'); // Erase character from screen
                  } else {
                    term?.write(data); // Echo back the typed data
                  }
        
                  if (data === '\r') { // Enter key pressed
                    // Emulate command execution and response
                    setTimeout(() => {
                      if (term) {
                        // Get the current line content *before* writing the newline response
                        const currentLineY = term.buffer.active.cursorY;
                        const commandLine = term.buffer.active.getLine(currentLineY);
                        // Use translateToString(true) to trim whitespace automatically
                        const command = commandLine?.translateToString(true).trim();

                        // Write the newline *after* capturing the command
                        term.write('\r\n');

                        if (command === '$ show ip int br' || command === '$ show ip int brief') {
                          term.write('Interface      IP-Address   Status      Protocol\r\n');
                          term.write('GigabitEthernet0/0 192.168.1.1  up          up      \r\n');
                          term.write('GigabitEthernet0/1 unassigned   administratively down down    \r\n');
                          term.write('GigabitEthernet0/2 unassigned   administratively down down    \r\n');
                        } else if (command) { // Check if command is not empty
                          term.write(`Unknown command: ${command}\r\n`); // More informative default
                        }
                        // If command is empty (just Enter pressed), do nothing extra before the prompt
                      } else {
                        // Fallback if term is somehow null, though unlikely here
                         return; // Just return, the prompt is written later if term exists
                      }
                      term?.write('$ '); // Write prompt
                    }, 50); // Slightly reduced timeout
                  }
                });

        // --- Handle Resize ---
        const parentElement = terminalContainer.parentElement;
        if (parentElement) {
          resizeObserver = new ResizeObserver(() => {
            setTimeout(() => {
              try {
                // Check if term and fitAddon are initialized
                if (term && fitAddon) {
                  fitAddon.fit();
                }
              } catch (e) {
                console.error("Error fitting terminal on resize:", e);
              }
            }, 100);
          });
          resizeObserver.observe(parentElement);
        } else {
          console.warn("Terminal container parent element not found for ResizeObserver.");
        }

      } catch (error) {
          console.error("Failed to load or initialize xterm:", error);
          // Optionally display an error message to the user in the terminal container
          if (terminalContainer) {
              terminalContainer.innerText = "Error loading terminal.";
          }
      }
    } else if (!browser) {
        console.log("Skipping xterm initialization during SSR.");
    } else {
        console.error("Terminal container element not found on mount.");
    }
  });

  // --- Cleanup Logic ---
  onDestroy(() => {
    // onDestroy also only runs in the browser
    console.log("Destroying Terminal component: Disposing terminal and disconnecting observer.");
    if (resizeObserver) {
      // No need to check parentElement here, just disconnect
      resizeObserver.disconnect();
    }
    if (term) {
      term.dispose(); // Dispose of the xterm instance
    }
    // Nullify references
    term = null;
    fitAddon = null;
    resizeObserver = null;
  });

  // --- Optional: Expose methods ---
  // export function writeToTerminal(data: string) {
  //   if (term) { // Check if term is initialized
  //     term.write(data);
  //   }
  // }

</script>

<style>
  .terminal-container {
    width: 100%;
    height: 100%; /* Make height flexible to fill parent */
    overflow: hidden; /* Prevent scrollbars on this container */
    background-color: #1e1e1e; /* Match terminal theme background */
    padding: 5px; /* Add a small padding around the terminal */
    box-sizing: border-box; /* Include padding in size */
    border-radius: 4px; /* Optional: slightly rounded corners */
    position: relative; /* Needed for potential absolute positioning of overlays */
  }

  /* Ensure the xterm element itself fills the container */
  /* Use :global() to target elements generated by xterm */
  :global(.terminal.xterm) {
      height: 100% !important; /* Override default xterm styles */
      width: 100% !important;
      padding: 0; /* Remove padding from xterm itself if container has it */
      box-sizing: border-box;
      /* Ensure terminal is focusable */
      outline: none;
  }

  /* Ensure the viewport (scrolling area) behaves correctly */
  :global(.xterm-viewport) {
      width: 100% !important; /* Ensure viewport takes full width */
      overflow-y: auto !important; /* Allow vertical scrolling within xterm */
      box-sizing: border-box;
  }

  /* Ensure the screen (where text is rendered) uses full height */
   :global(.xterm-screen) {
       height: 100% !important;
       width: 100% !important;
       box-sizing: border-box;
   }

</style>

<div bind:this={terminalContainer} class="terminal-container" tabindex="0"></div>

