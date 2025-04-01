<script lang="ts">
  import TerminalEmulator from '$lib/terminal/Terminal.svelte';
  import QuickConnect from '$lib/Connection Tab/QuickConnect.svelte';
  import AIAgent from '$lib/AIAgent/AIAgent.svelte'; // Import the new component
  import { runCommand } from '$lib/Connection Tab/ConnectionStore';
  import { onMount } from 'svelte';
  
  // Reference to the terminal component
  let terminalComponent: TerminalEmulator;
  let activeConnectionId = $state<string | null>(null);
  
  // Handle when a connection is selected
  function handleConnectionSelect(connectionId: string) {
    console.log('Connection selected:', connectionId);
    activeConnectionId = connectionId;
    
    // Set up the terminal to use the selected connection
    if (terminalComponent) {
      // Set the command handler to use our connection
      terminalComponent.setCommandHandler(async (command: string) => {
        try {
          console.log('Executing command:', command);
          
          if (!activeConnectionId) {
            return "No active connection";
          }
          
          // Ensure we're not getting $ in the command
          const cleanCommand = command.startsWith('$') 
            ? command.substring(1).trim() 
            : command;
          
          // Execute the command on the selected connection and return the response
          const result = await runCommand(cleanCommand);
          return result;
        } catch (error) {
          console.error('Error executing command:', error);
          return `Error: ${error}`;
        }
      });
      
      // Clear the terminal and add a welcome message
      terminalComponent.writeToTerminal("\r\n\x1b[1mConnected to SSH session. You can now type commands.\x1b[0m\r\n\r\n$ ");
    }
  }
  
  onMount(() => {
    // Initial setup for terminal if needed
    if (terminalComponent && !activeConnectionId) {
      terminalComponent.writeToTerminal("Welcome to the terminal. Select a connection to begin.\r\n");
    }
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
  <QuickConnect onSelectConnection={handleConnectionSelect} />

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