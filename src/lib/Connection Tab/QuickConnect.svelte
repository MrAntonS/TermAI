<script lang="ts">
  import { connect } from './DummyConnectionAgent';
  import { v4 as uuidv4 } from 'uuid';

  let connections: { id: string; name: string; type: string }[] = [
    { id: uuidv4(), name: 'Router-Core-1', type: 'SSH' },
    { id: uuidv4(), name: 'Switch-Access-5', type: 'Telnet' },
  ];

  let loading = false;

  async function quickConnect() {
    loading = true;
    const result = await connect();
    console.log(result);
    loading = false;

    // Add a new connection
    connections = [...connections, { id: uuidv4(), name: result, type: 'Dummy' }];
  }
</script>

<aside class="w-64 p-4 flex flex-col">
  <button
    on:click={quickConnect}
    class="w-full text-green-400 font-semibold py-2 px-4 rounded-md mb-4 flex items-center justify-center hover:bg-gray-900 transition"
  >
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 24" width="20" height="24" fill="currentColor" class="mr-2">
      <title>Lightning Bolt</title>
      <path d="M10 0 L6 12 L12 12 L8 24 L14 12 L10 12 Z" />
    </svg>
    Quick Connect
  </button>

  <h2 class="text-xs font-semibold uppercase tracking-wide mb-2 pl-2">Connections</h2>
  <div class="flex-grow overflow-y-auto pr-2">
    {#each connections as connection}
      <a href="#" class="flex items-center p-2 rounded-md text-sm font-medium text-green-400 hover:bg-gray-900">
        <svg class="w-5 h-5 mr-3" fill="currentColor" viewBox="0 0 20 20" xmlns="http://www.w3.org/2000/svg">
          <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm0-2a6 6 0 100-12 6 6 0 000 12z" clip-rule="evenodd"></path>
        </svg>
        {connection.name} ({connection.type})
      </a>
    {/each}
    {#if loading}
      <a href="#" class="flex items-center p-2 rounded-md text-sm font-medium text-green-400 hover:bg-gray-900">
        <div class="loader"></div>
      </a>
    {/if}
  </div>
</aside>