<script lang="ts">
  import TerminalEmulator from '$lib/terminal/Terminal.svelte';
  import QuickConnect from '$lib/Connection Tab/QuickConnect.svelte';
  import AIAgent from '$lib/AIAgent/AIAgent.svelte'; // Import the new component
  import { runCommand } from '$lib/Connection Tab/ConnectionStore';
  import { onMount } from 'svelte';
  
  // Reference to the terminal component
  let terminalComponent: TerminalEmulator;
  // Removed activeConnectionId state, as TerminalEmulator manages its own connection state internally.
  
  // Handle when a connection is selected
  function handleConnectionSelect(connectionId: string) {
    // This function is called when a connection is selected from the QuickConnect list.
    // The previous logic calling `setCommandHandler` is removed as it's no longer
    // needed; Terminal.svelte handles its input/output directly via Tauri invokes/events.
    console.log('Connection selected in list (ID might be internal store ID):', connectionId);

    // TODO: Implement logic if selecting an existing connection from the list should
    // re-establish the connection in the terminal. This would involve:
    // 1. Getting connection details from ConnectionStore using `connectionId`.
    // 2. Calling `terminalComponent.connect(details)`.
    // For now, selecting from the list doesn't re-initiate the connection in the terminal.
    // The active connection state is primarily managed by the Terminal component itself
    // after being initiated by the QuickConnectModal.
  }
  
  onMount(() => {
    // Initial setup for terminal if needed
    // The TerminalEmulator component handles its own initial welcome message.
    // No specific action needed here unless we want to override it.
    // if (terminalComponent) {
    //   // Example: terminalComponent.writeToTerminal("Page loaded.\r\n");
    // }
  });
</script>

<svelte:head>
  <link rel="stylesheet" href="https://unpkg.com/xterm/css/xterm.css" />
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous">
  <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet">
</svelte:head>

<div class="h-screen flex overflow-hidden bg-black text-green-500 font-inter">
  <!-- Sidebar -->
  <QuickConnect
    onSelectConnection={handleConnectionSelect}
    terminalConnectFn={terminalComponent?.connect}
  />

  <!-- Main Content -->
  <main class="flex-1 flex p-4 overflow-hidden space-x-4">
    <!-- Terminal takes up remaining space -->
    <div class="flex-1 flex flex-col overflow-hidden">
       <TerminalEmulator bind:this={terminalComponent} />
    </div>
    <!-- AI Agent Panel -->
    <AIAgent />
  </main>
</div>