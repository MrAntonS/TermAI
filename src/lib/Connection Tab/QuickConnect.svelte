<script lang="ts">
  import { v4 as uuidv4 } from 'uuid';
  import QuickConnectModal from './QuickConnectModal.svelte';

  // Define Connection type (adjust payload if needed)
  type ConnectionDetails = {
      hostname: string;
      port: number;
      username: string;
      authMethod: 'password' | 'key';
  };
  type Connection = {
      id: string;
      name: string;
      type: string;
      details?: ConnectionDetails; // Optional place to store more details
  };

  // Use $state for reactive variables in Svelte 5
  let connections = $state<Connection[]>([
    { id: uuidv4(), name: 'Router-Core-1', type: 'SSH' },
    { id: uuidv4(), name: 'Switch-Access-5', type: 'Telnet' },
  ]);

  let loading = $state(false);
  let isModalOpen = $state(false);

  function openQuickConnectModal() {
    isModalOpen = true;
  }

  function closeQuickConnectModal() {
    isModalOpen = false;
  }

  // Handler for the new connection data passed via prop callback
  function handleNewConnection(newConnection: Connection) {
      console.log('Received new connection data via prop:', newConnection);

      // Add the new connection to the array (Svelte 5 reactivity handles update)
      connections.unshift(newConnection); // Add to beginning
      // Or: connections.push(newConnection); // Add to end

      // Modal is closed by its own handleSubmit/closeModal now
  }

</script>

<aside class="w-64 p-4 flex flex-col bg-gray-800 text-gray-200 h-screen">
  <button
    onclick={openQuickConnectModal}
    class="w-full bg-green-600 text-white font-semibold py-2 px-4 rounded-md mb-4 flex items-center justify-center hover:bg-green-700 transition"
    disabled={loading}
  >
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 24" width="20" height="24" fill="currentColor" class="mr-2">
      <title>Lightning Bolt</title>
      <path d="M10 0 L6 12 L12 12 L8 24 L14 12 L10 12 Z" />
    </svg>
    Quick Connect
  </button>

  <h2 class="text-xs font-semibold uppercase text-gray-400 tracking-wide mb-2 pl-2">Connections</h2>
  <div class="flex-grow overflow-y-auto pr-2 -mr-2">
    {#each connections as connection (connection.id)}
      <a href="#" class="flex items-center p-2 rounded-md text-sm font-medium text-gray-300 hover:bg-gray-700 hover:text-white group">
        <svg class="w-5 h-5 mr-3 text-gray-400 group-hover:text-gray-300" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"></path></svg>
        <span class="truncate">{connection.name} ({connection.type})</span>
      </a>
    {/each}
    {#if loading}
      <div class="flex items-center p-2 text-sm font-medium text-gray-400">
        <div class="loader mr-2"></div>
        Loading...
      </div>
    {/if}
  </div>
</aside>

{#if isModalOpen}
  <QuickConnectModal
    onClose={closeQuickConnectModal}      
    onNewConnection={handleNewConnection}
  />
{/if}

<style>
  /* Loader styles */
  .loader {
    border: 4px solid #555;
    border-top: 4px solid #3498db;
    border-radius: 50%;
    width: 16px;
    height: 16px;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  /* Scrollbar styles */
  div::-webkit-scrollbar {
    width: 6px;
  }
  div::-webkit-scrollbar-track {
    background: #2d3748;
    border-radius: 3px;
  }
  div::-webkit-scrollbar-thumb {
    background: #4a5568;
    border-radius: 3px;
  }
  div::-webkit-scrollbar-thumb:hover {
    background: #718096;
  }
</style>