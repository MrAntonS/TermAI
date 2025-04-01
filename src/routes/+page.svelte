<script lang="ts">
  import TerminalEmulator from '$lib/terminal/Terminal.svelte';
  import QuickConnect from '$lib/QuickConnect.svelte';
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
  <main class="flex-1 flex flex-col p-4 overflow-hidden">
    <TerminalEmulator />
  </main>
</div>