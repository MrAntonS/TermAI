<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core'; // Import invoke
  let aiTextareaElement: HTMLTextAreaElement;
  let aiContentElement: HTMLDivElement;
  let question = '';
  // Add 'error' type for messages
  let messages: { type: 'user' | 'ai' | 'system' | 'suggestion' | 'explanation' | 'error', content: string, title?: string }[] = [
    { type: 'system', content: 'AI chat interface initialized. Ready for input.' },
    // Example messages removed for brevity, can be added back if needed
    // { type: 'suggestion', title: 'Suggestion:', content: 'Check interface status: <code>show ip interface brief</code>' },
    // { type: 'explanation', title: 'Explanation:', content: 'OSPF state \'FULL\' means successful adjacency.' }
  ];
  let isLoading = false; // Add loading state

  async function sendMessage() { // Make function async
    const userQuestion = question.trim();
    if (userQuestion === '' || isLoading) {
      return; // Don't send empty messages or while loading
    }

    // Add user message immediately
    messages = [...messages, { type: 'user', content: userQuestion }];
    question = ''; // Clear input
    isLoading = true; // Set loading state

    // Scroll after adding user message
    scrollToBottom();

    try {
      // Call the backend command
      const responseText = await invoke<string>('send_to_gemini', { prompt: userQuestion });
      messages = [...messages, { type: 'ai', content: responseText }];
    } catch (error) {
      console.error('Error calling Gemini:', error);
      const errorMessage = typeof error === 'string' ? error : 'An unknown error occurred.';
      messages = [...messages, { type: 'error', content: `Error: ${errorMessage}` }];
    } finally {
      isLoading = false; // Reset loading state
      // Scroll after adding AI response or error
      scrollToBottom();
    }
  }

  // Helper function to scroll chat to bottom
  function scrollToBottom() {
      // Use timeout to ensure DOM has updated
      setTimeout(() => {
        if (aiContentElement) {
          aiContentElement.scrollTop = aiContentElement.scrollHeight;
        }
      }, 0);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault(); // Prevent default newline on Enter
      sendMessage();
    }
  }

  onMount(() => {
    // Initial scroll to bottom if needed
    if (aiContentElement) {
      scrollToBottom(); // Use helper function
    }
  });

</script>

<section class="ai-panel">
  <div class="panel-header flex items-center justify-between">
    <h2>AI Assistant</h2>
    <button title="Toggle AI Panel Visibility" class="p-1 rounded-full hover:bg-gray-600">
      <!-- Using a simple SVG placeholder for now -->
      <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 20 20"><path d="M10 12a2 2 0 100-4 2 2 0 000 4z"></path><path fill-rule="evenodd" d="M.458 10C1.732 5.943 5.522 3 10 3s8.268 2.943 9.542 7c-1.274 4.057-5.064 7-9.542 7S1.732 14.057.458 10zM14 10a4 4 0 11-8 0 4 4 0 018 0z" clip-rule="evenodd"></path></svg>
    </button>
  </div>
  <div class="ai-content" bind:this={aiContentElement}>
    {#each messages as message}
      {#if message.type === 'system'}
        <div class="ai-message">{message.content}</div>
      {:else if message.type === 'user'}
        <div class="ai-message"><b>User:</b> {message.content}</div>
      {:else if message.type === 'ai'}
        <div class="ai-message"><b>AI:</b> {message.content}</div>
      {:else if message.type === 'suggestion'}
        <div class="ai-suggestion">
          <p class="ai-suggestion-title">{message.title}</p>
          <p class="ai-suggestion-content">{@html message.content}</p>
        </div>
      {:else if message.type === 'explanation'}
        <div class="ai-explanation">
          <p class="ai-explanation-title">{message.title}</p>
          <p class="ai-explanation-content">{message.content}</p>
        </div>
      {:else if message.type === 'error'}
        <div class="ai-message ai-error"><b>System Error:</b> {message.content}</div>
      {/if}
    {/each}
  </div>
  <div class="ai-input-area">
    <textarea
      rows="3"
      class="ai-textarea"
      placeholder="Ask the AI..."
      bind:value={question}
      on:keydown={handleKeydown}
      bind:this={aiTextareaElement}
    ></textarea>
    <button class="ai-send-button" on:click={sendMessage} disabled={isLoading}>
      {#if isLoading}
        <span>Sending...</span>
      {:else}
        <span>Send</span>
      {/if}
    </button>
  </div>
</section>

<style>
  /* Color Palette (assuming these are defined globally or passed as props) */
  :root {
    --color-primary-black: #0f0f0f;
    --color-border-gray: #36454f;
    --color-text-white: #468f46;
    --color-hover-bg: #4a5560;
    --color-panel-bg: #0f0f0f;
  }

  /* General text styles (might be inherited) */
  h2, p, button, div, section, label, span {
    color: var(--color-text-white) !important;
    text-shadow: 0 0 5px rgba(255, 255, 250, 0.3);
  }
  .panel-header h2 {
     text-shadow: none;
  }

  /* Inputs and Textareas */
  textarea, input {
    color: var(--color-text-white) !important;
    background-color: var(--color-primary-black) !important;
  }
   textarea:focus, input:focus {
     outline: none;
     box-shadow: 0 0 0 1px var(--color-hover-bg) !important;
   }

  /* Panel styles */
   .panel {
     border-radius: 0.5rem;
     box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.1);
     overflow: hidden;
     display: flex;
     flex-direction: column;
   }

  .panel-header {
    padding: 0.5rem 1rem;
    color: var(--color-text-white) !important;
    font-size: 0.875rem;
    font-weight: 600;
    margin-bottom: 0.5rem;
    flex-shrink: 0;
  }

  /* AI Assistant panel */
  .ai-panel {
    width: 33.333333%; /* Or adjust as needed */
    display: flex;
    flex-direction: column;
    background-color: var(--color-panel-bg);
    border-radius: 0.5rem;
    box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.1);
    overflow: hidden;
    height: 100%; /* Make panel fill height */
  }

  .ai-content {
    flex: 1;
    padding: 0 1rem 1rem 1rem;
    overflow-y: auto;
    /* Custom scrollbar styling */
    scrollbar-width: thin;
    scrollbar-color: var(--color-border-gray) var(--color-primary-black);
  }
  .ai-content::-webkit-scrollbar {
    width: 8px;
  }
  .ai-content::-webkit-scrollbar-track {
    background: var(--color-primary-black);
  }
  .ai-content::-webkit-scrollbar-thumb {
    background-color: var(--color-border-gray);
    border-radius: 4px;
  }
  .ai-content::-webkit-scrollbar-thumb:hover {
    background-color: var(--color-hover-bg);
  }


  .ai-message {
    margin-bottom: 1rem;
    color: var(--color-text-white) !important;
    font-size: 0.875rem;
    /* font-style: italic; Removed italic for user/ai messages */
    word-wrap: break-word; /* Ensure long words wrap */
  }
  .ai-message b { /* Style for User/AI labels */
      font-weight: 600;
  }
  .ai-message:first-child { /* Style for initial system message */
      font-style: italic;
  }
  .ai-error { /* Style for error messages */
      color: #f87171 !important; /* Tailwind red-400 */
      font-weight: bold;
  }


  .ai-suggestion, .ai-explanation {
    background-color: var(--color-border-gray);
    padding: 0.75rem;
    border-radius: 0.375rem;
    margin-bottom: 1rem;
    box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
  }

  .ai-suggestion-title, .ai-explanation-title {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--color-text-white) !important;
    margin-bottom: 0.25rem;
    text-shadow: none;
  }

  .ai-suggestion-content, .ai-explanation-content {
    font-size: 0.875rem;
    color: var(--color-text-white) !important;
    text-shadow: none;
  }
   .ai-suggestion-content code { /* Ensure code within suggestions is styled */
      background-color: var(--color-primary-black);
      padding: 0.1rem 0.3rem;
      border-radius: 0.25rem;
      font-size: 0.9em; /* Slightly larger than default code */
      color: var(--color-text-white) !important;
      text-shadow: none;
   }


  .ai-input-area {
    margin-top: auto;
    padding: 1rem;
    background-color: var(--color-panel-bg);
    flex-shrink: 0;
    border-top: 1px solid rgba(255, 255, 255, 0.1); /* Add subtle separator */
  }

  .ai-textarea {
    width: 100%;
    background-color: var(--color-primary-black) !important;
    border: 1px solid var(--color-border-gray); /* Add border */
    border-radius: 0.375rem;
    padding: 0.5rem;
    font-size: 0.875rem;
    color: var(--color-text-white) !important;
    resize: none;
    margin-bottom: 0.5rem; /* Add space below textarea */
  }

  .ai-send-button {
    width: 100%;
    background-color: var(--color-border-gray);
    color: var(--color-text-white) !important;
    font-weight: 600;
    padding: 0.375rem 0.75rem;
    border-radius: 0.375rem;
    font-size: 0.875rem;
    transition: all 150ms ease-in-out;
    box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
    border: none;
    cursor: pointer; /* Add pointer cursor */
  }

  .ai-send-button:hover {
    background-color: var(--color-hover-bg);
  }
  .ai-send-button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  /* Ensure code blocks are styled correctly */
  code {
    background-color: var(--color-primary-black);
    padding: 0.1rem 0.3rem;
    border-radius: 0.25rem;
    font-size: 0.8em;
    color: var(--color-text-white) !important;
    text-shadow: none;
  }

  /* Tailwind utility classes used in the template */
  .flex { display: flex; }
  .items-center { align-items: center; }
  .justify-between { justify-content: space-between; }
  .p-1 { padding: 0.25rem; }
  .rounded-full { border-radius: 9999px; }
  .hover\:bg-gray-600:hover { background-color: #4a5568; } /* Example gray */
  .w-4 { width: 1rem; }
  .h-4 { height: 1rem; }

</style>