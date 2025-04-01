<script lang="ts">
  import { v4 as uuidv4 } from 'uuid';
  import { tick } from 'svelte'; // Import tick for focusing
  import QuickConnectModal from './QuickConnectModal.svelte';
  import { activeConnections, currentConnectionId, setCurrentConnection, disconnect, type ActiveConnection } from './ConnectionStore';
  
  // Props for terminal control events
  type $$Props = {
    onSelectConnection?: (connectionId: string) => void;
  };
  
  // Get props using $props rune
  const { onSelectConnection } = $props();
  
  // --- State ---
  // Use the activeConnections store instead of local state
  let connections = $derived(Array.from($activeConnections));

  let loading = $state(false);
  let isModalOpen = $state(false);

  // State for editing
  let editingConnectionId = $state<string | null>(null);
  let editingName = $state('');
  let inputElement: HTMLInputElement | null = null; // To hold reference to the input element for focus

  // --- Modal Functions ---
  function openQuickConnectModal() {
    isModalOpen = true;
  }

  function closeQuickConnectModal() {
    isModalOpen = false;
  }

  function handleNewConnection(newConnection: any) {
      console.log('Received new connection data via prop:', newConnection);
      // The connection is already added to the store in QuickConnectModal
      // Here we just notify the parent about the selected connection if callback is provided
      if (onSelectConnection && newConnection.connection_id) {
          onSelectConnection(newConnection.connection_id);
      }
  }

  // --- Renaming Functions ---
  // Handle connection selection
  function selectConnection(connection: ActiveConnection) {
    if (editingConnectionId === connection.id) {
      return; // Don't select while editing
    }
    
    console.log('Selecting connection:', connection);
    setCurrentConnection(connection.id);
    
    // Notify parent component if callback exists
    if (onSelectConnection) {
      onSelectConnection(connection.connectionId);
    }
  }
  
  async function startEditing(connection: ActiveConnection) {
    editingConnectionId = connection.id;
    editingName = connection.name;
    // Wait for the DOM to update, then focus the input
    await tick();
    inputElement?.focus();
    inputElement?.select(); // Optional: select text on focus
  }

  function saveEdit() {
    if (!editingConnectionId) return;

    const trimmedName = editingName.trim();
    if (trimmedName) { // Only save if not empty after trimming
        const index = connections.findIndex(c => c.id === editingConnectionId);
        if (index !== -1) {
            // Create a new object to ensure reactivity if needed, or modify directly
            // Direct modification works fine with $state arrays in S5
            connections[index].name = trimmedName;
        }
    }
    // If name was empty/whitespace only, it effectively cancels the edit without saving

    cancelEdit(); // Exit editing mode
  }

  function cancelEdit() {
    editingConnectionId = null;
    editingName = ''; // Clear temp name
    inputElement = null; // Clear element reference
  }

  function handleInputKeyDown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      saveEdit();
    } else if (event.key === 'Escape') {
      cancelEdit();
    }
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
      <a href="#"
         class="flex items-center p-2 rounded-md text-sm font-medium 
                {$currentConnectionId === connection.id 
                  ? 'bg-gray-700 text-white' 
                  : 'text-gray-300 hover:bg-gray-700 hover:text-white'} 
                group"
         onclick={() => selectConnection(connection)}
         ondblclick={() => startEditing(connection)}
      >
        <svg class="w-5 h-5 mr-3 text-gray-400 group-hover:text-gray-300 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"></path></svg>

        {#if editingConnectionId === connection.id}
          <input
            type="text"
            bind:this={inputElement} 
            bind:value={editingName}  
            onblur={saveEdit}         
            onkeydown={handleInputKeyDown} 
            class="flex-grow bg-gray-600 text-white text-sm p-0 border border-blue-400 rounded outline-none focus:ring-1 focus:ring-blue-300"
            autocomplete="off"
          />
        {:else}
          <span class="truncate">{connection.name} ({connection.type})</span>
        {/if}
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

  /* Simple style for the edit input to make it fit better */
  input[type="text"] {
      /* Minimal styling, adjust as needed */
      height: 1.5em; /* Match text height */
      line-height: 1.5em;
      padding: 0 0.25em; /* Minimal padding */
      margin-left: -0.25em; /* Align with text */
  }
</style>