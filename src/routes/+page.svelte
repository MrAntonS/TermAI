<script lang="ts">
  import TerminalEmulator from '$lib/terminal/Terminal.svelte';
  import QuickConnect from '$lib/Connection Tab/QuickConnect.svelte';
  import AIAgent from '$lib/AIAgent/AIAgent.svelte'; // Import the new component
</script>

<svelte:head>
  <link rel="stylesheet" href="https://unpkg.com/xterm/css/xterm.css" />
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous">
  <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet">
</svelte:head>

<div class="h-screen flex overflow-hidden bg-black text-green-500 font-inter">
  <!-- Sidebar -->
  <QuickConnect />

  <!-- Main Content -->
  <main class="flex-1 flex p-4 overflow-hidden space-x-4">
    <!-- Terminal takes up remaining space -->
    <div class="flex-1 flex flex-col overflow-hidden">
       <TerminalEmulator />
    </div>
    <!-- AI Agent Panel -->
    <AIAgent />
  </main>
</div>