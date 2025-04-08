<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type Terminal from '../terminal/Terminal.svelte';
  // Import prompt generation functions
  import { getInitialPrompt, getPromptAfterAcceptedCommand, getPromptAfterRejectedCommand, getContinuationPrompt } from './prompts';

  // --- Props ---
  export let terminalInstance: Terminal | null = null; // Prop to receive Terminal instance

  // --- State ---
  let aiTextareaElement: HTMLTextAreaElement;
  let aiContentElement: HTMLDivElement;
  let question = '';
  // Add 'error', 'confirmation' types for messages
  type MessageType = 'user' | 'ai' | 'system' | 'suggestion' | 'explanation' | 'error' | 'debug' | 'confirmation';
  // Add optional 'thinking' field for AI messages
  type Message = { type: MessageType, content: string, thinking?: string, title?: string, commands?: string[], onAccept?: () => void, onReject?: () => void };
  let messages: Message[] = [
    { type: 'system', content: 'AI chat interface initialized. Ready for input.' },
    // Example messages removed for brevity
  ];
  let isLoading = false; // Add loading state
  let pendingCommands: string[] = []; // Commands awaiting confirmation
  let showConfirmation = false; // Flag to show confirmation UI (Now managed via message type)
  let confirmationResolve: ((accepted: boolean) => void) | null = null; // To resolve the confirmation promise
  let isWaitingForUser = false; // Flag to indicate AI is waiting for user input

  const MAX_AI_STEPS = 5; // Safety limit for interaction loop
  async function sendMessage() {
    const userQuestion = question.trim();
    if (userQuestion === '' || isLoading) return; // Don't send empty or while processing

    // --- Resume Interaction if AI was Waiting ---
    if (isWaitingForUser) {
        console.log("AI Agent: Resuming interaction after user input.");
        isWaitingForUser = false; // Reset the flag
        isLoading = true; // Set loading state
        messages = [...messages, { type: 'user', content: userQuestion }]; // Add user message
        question = ''; // Clear input
        scrollToBottom();

        // --- Gather context for continuation ---
        let terminalContent: string[] = [];
        if (terminalInstance) {
            try {
                terminalContent = terminalInstance.getTerminalContent();
            } catch (err) {
                console.error("AI Agent: Error reading terminal content on resume:", err);
                messages = [...messages, { type: 'error', content: `Error reading terminal on resume: ${err}` }];
                isLoading = false;
                scrollToBottom();
                return;
            }
        }
        const terminalContext = terminalContent.slice(-10).join('\n');
        const historyLimit = 50;

        // --- Construct Continuation Prompt ---
        // Use getContinuationPrompt, ensuring it includes the latest user message from the history
        const continuationPrompt = getContinuationPrompt(messages, historyLimit, terminalContext);
        console.log("AI Agent: Constructed continuation prompt:\n---\n" + continuationPrompt + "\n---");

        // --- Restart Interaction Loop ---
        try {
            // Decide on the step number - perhaps reset or use a marker? Resetting for now.
            await runAIInteractionLoop(continuationPrompt, 0);
        } catch (error) {
            console.error('AI Interaction Loop Error on Resume:', error);
            const errorMessage = typeof error === 'string' ? error : 'An unexpected error occurred during AI interaction resume.';
            messages = [...messages, { type: 'error', content: `Interaction Error: ${errorMessage}` }];
        } finally {
            // isLoading should be managed within the loop now, but set false just in case
            // isLoading = false; // Let the loop handle this
            scrollToBottom();
        }
        return; // Stop further execution in sendMessage
    }

    // --- Start New Interaction ---
    // (Original sendMessage logic starts here)
    messages = [...messages, { type: 'user', content: userQuestion }];

    question = '';
    isLoading = true;
    scrollToBottom();

    // --- Get Initial Terminal Context ---
    let initialTerminalContent: string[] = [];
    if (terminalInstance) {
        try {
            initialTerminalContent = terminalInstance.getTerminalContent();
        } catch (err) {
            console.error("AI Agent: Error reading initial terminal content:", err);
            messages = [...messages, { type: 'error', content: `Error reading terminal: ${err}` }];
            isLoading = false;
            scrollToBottom();
            return; // Stop if we can't read terminal
        }
    } else {
        console.warn("AI Agent: Terminal instance not available for initial read.");
    }
    // --- Add Debug Message for Initial Read ---
    //messages = [...messages, { type: 'debug', title: 'DEBUG: Initial Terminal Read', content: `\`\`\`\n${initialTerminalContent.join('\n')}\n\`\`\`` }];
    scrollToBottom(); // Scroll after adding debug message

    // --- Construct Initial Prompt ---
    const initialTerminalContext = initialTerminalContent.slice(-10).join('\n'); // Get last 10 lines
    console.log("AI Agent: Extracted initial terminal context for prompt:\n---\n" + initialTerminalContext + "\n---");

    // --- Gather recent conversation history ---
    const historyLimit = 50; // Include last N messages (user/ai)

    // Use imported function to generate the initial prompt
    const initialPrompt = getInitialPrompt(messages, historyLimit, initialTerminalContext, userQuestion);
    console.log("AI Agent: Constructed initial prompt:\n---\n" + initialPrompt + "\n---");

    // --- Start Interaction Loop ---
    try {
        await runAIInteractionLoop(initialPrompt, 0); // Start the loop
    } catch (error) {
        console.error('AI Interaction Loop Error:', error);
        const errorMessage = typeof error === 'string' ? error : 'An unexpected error occurred during the AI interaction.';
        messages = [...messages, { type: 'error', content: `Interaction Error: ${errorMessage}` }];
    } finally {
        isLoading = false; // Ensure loading state is reset
        scrollToBottom();
    }
  }

  // --- Recursive Interaction Function ---
  async function runAIInteractionLoop(prompt: string, step: number) {
    // Removed commandsWereExecuted, newTerminalContent, commands declarations from here, handled within command execution logic
    if (step >= MAX_AI_STEPS) {
        messages = [...messages, { type: 'error', content: `Max interaction steps (${MAX_AI_STEPS}) reached. Stopping.` }];
        scrollToBottom();
        throw new Error("Max interaction steps reached."); // Stop the loop
    }

    console.log(`AI Agent: Step ${step + 1} - Sending prompt:`, prompt);
    const rawResponse = await invoke<string>('send_to_gemini', { prompt: prompt });
    console.log(`AI Agent: Step ${step + 1} - Received raw response:`, rawResponse);

    // --- Add Debug Message for Raw AI Response ---
   // messages = [...messages, { type: 'debug', title: `DEBUG: Raw AI Response (Step ${step + 1})`, content: rawResponse }];
    scrollToBottom(); // Scroll after adding debug message

    // --- Process Response ---
    let aiResponseText = rawResponse;
    let aiThinkingText: string | undefined = undefined; // Variable for thinking

    // Extract thinking first
    const thinkingMatch = rawResponse.match(/<thinking>(.*?)<\/thinking>/s);
    if (thinkingMatch && thinkingMatch[1]) {
        aiThinkingText = thinkingMatch[1].trim();
        // Remove thinking tag from the main response text
        aiResponseText = aiResponseText.replace(/<thinking>.*?<\/thinking>/s, '').trim();
    }

    // Now process commands and task completion on the remaining text
    const cmdMatch = aiResponseText.match(/<cmd>(.*?)<\/cmd>/s); // Use 's' flag for multiline
    const taskCompleteMatch = aiResponseText.includes('<task_complete/>');
    const waitForUserMatch = aiResponseText.includes('<wait_for_user/>'); // Check for wait tag

    // Extract text part (remove cmd, task_complete, and wait_for_user tags for display)
    if (cmdMatch) {
        aiResponseText = aiResponseText.replace(/<cmd>.*?<\/cmd>/s, '').trim(); // Use 's' flag
    }
    if (taskCompleteMatch) {
        aiResponseText = aiResponseText.replace(/<task_complete\/>/, '').trim();
    }
    if (waitForUserMatch) { // Remove wait tag
        aiResponseText = aiResponseText.replace(/<wait_for_user\/>/, '').trim();
    }
    aiResponseText = aiResponseText.trim(); // Final trim

    // Display textual response from AI, including thinking if present
    if (aiResponseText || aiThinkingText) { // Add message if either thinking or content exists
        messages = [...messages, { type: 'ai', content: aiResponseText, thinking: aiThinkingText }]; // Add thinking here
        scrollToBottom();
    }

    // --- Handle Task Completion ---
    if (taskCompleteMatch) {
        messages = [...messages, { type: 'system', content: 'AI indicates task completed.' }];
        scrollToBottom();
        isLoading = false; // Task complete, stop loading
        return; // End the loop
    }

    // --- Handle Commands ---
    if (cmdMatch && cmdMatch[1]) {
        const commandsToExecuteStr = cmdMatch[1].trim();
        const extractedCommands = commandsToExecuteStr.split('\n').map(cmd => cmd.trim()).filter(cmd => cmd.length > 0);

        if (extractedCommands.length > 0 && terminalInstance) {
            // --- Show Confirmation ---
            const accepted = await promptForCommandConfirmation(extractedCommands);

            if (accepted) {
                // --- User Accepted ---
                messages = [...messages, { type: 'system', content: `Executing command(s):\n\`\`\`\n${extractedCommands.join('\n')}\n\`\`\`` }];
                scrollToBottom();

                let newTerminalContent: string[] = []; // Scope to this block
                try {
                    // Execute commands
                    for (const command of extractedCommands) {
                        console.log(`AI Agent: Invoking ai_write_to_ssh with: ${command}\\n`);
                        await invoke('ai_write_to_ssh', { data: command + '\n' });
                        await new Promise(resolve => setTimeout(resolve, 150)); // Short delay
                    }

                    // Wait and read new terminal state
                    await new Promise(resolve => setTimeout(resolve, 1000)); // Wait 1 second
                    newTerminalContent = terminalInstance.getTerminalContent();
                    console.log("AI Agent: Read new terminal content after accepted command execution:", newTerminalContent);
                    // Optional Debug Message:
                    // messages = [...messages, { type: 'debug', title: `DEBUG: Terminal Read (After Accepted Cmds - Step ${step + 1})`, content: `\`\`\`\n${newTerminalContent.join('\n')}\n\`\`\`` }];
                    // scrollToBottom();

                } catch (cmdError) {
                    console.error(`Error during command execution or terminal read:`, cmdError);
                    const cmdErrorMessage = typeof cmdError === 'string' ? cmdError : `Failed during command execution or terminal read.`;
                    messages = [...messages, { type: 'error', content: `Command execution failed: ${cmdErrorMessage}` }];
                    scrollToBottom();
                    throw new Error(`Command execution/read failed.`); // Stop loop
                }

                // Construct next prompt based on executed commands
                const nextTerminalContext = newTerminalContent.slice(-10).join('\n');
                // Use imported function to generate the prompt after accepted commands
                const nextPrompt = getPromptAfterAcceptedCommand(nextTerminalContext);
                console.log(`AI Agent: Step ${step + 1} - Constructed next prompt (after accepted commands):\n---\n` + nextPrompt + "\n---");
                await runAIInteractionLoop(nextPrompt, step + 1); // Continue loop

            } else {
                // --- User Rejected ---
                messages = [...messages, { type: 'system', content: 'User rejected the proposed command(s).' }];
                scrollToBottom();

                // Construct next prompt indicating rejection
                let currentTerminalContent: string[] = [];
                 try {
                     currentTerminalContent = terminalInstance.getTerminalContent(); // Get current state as commands weren't run
                 } catch (err) {
                     console.error("AI Agent: Error reading terminal content after command rejection:", err);
                     messages = [...messages, { type: 'error', content: `Error reading terminal after command rejection: ${err}` }];
                     scrollToBottom();
                     throw new Error("Failed to read terminal after command rejection."); // Stop loop
                 }

                const nextTerminalContext = currentTerminalContent.slice(-10).join('\n');
                // Use imported function to generate the prompt after rejected commands
                const nextPrompt = getPromptAfterRejectedCommand(nextTerminalContext);
                console.log(`AI Agent: Step ${step + 1} - Constructed next prompt (after rejected commands):\n---\n` + nextPrompt + "\n---");
                await runAIInteractionLoop(nextPrompt, step + 1); // Continue loop
            }
            return; // Stop further processing in this step as the loop was continued recursively

        } else if (extractedCommands.length > 0 && !terminalInstance) {
            messages = [...messages, { type: 'error', content: `Cannot propose command(s): Terminal not available.` }];
            scrollToBottom();
            throw new Error("Terminal not available for command proposal."); // Stop loop
        }
        // If cmdMatch but extractedCommands array is empty, treat as no command.
    }

    // --- Handle Wait for User ---
    if (waitForUserMatch) {
        messages = [...messages, { type: 'system', content: 'AI is waiting for your response.' }];
        isWaitingForUser = true; // Set the flag
        isLoading = false; // Allow user input
        scrollToBottom();
        console.log("AI Agent: Pausing interaction, waiting for user input.");
        return; // Stop the loop here
    }

    // --- If task is not complete AND not waiting, continue the loop ---
    if (!taskCompleteMatch && !waitForUserMatch) { // Added !waitForUserMatch condition
        // Determine the terminal content to use for the next prompt
        let currentTerminalContent: string[] = [];
        // This block now only runs if NO commands were proposed by the AI in this step.
        // If commands *were* proposed, the logic is handled within the confirmation block above.
        // let currentTerminalContent: string[] = []; // Removed duplicate declaration
        if (terminalInstance) {
             // No commands proposed in this step, read the current state.
             try {
                 currentTerminalContent = terminalInstance.getTerminalContent();
                 console.log("AI Agent: Read terminal content (no commands proposed this step):", currentTerminalContent);
                 // Optional Debug Message:
                 // messages = [...messages, { type: 'debug', title: `DEBUG: Terminal Read (Step ${step + 1}, No Cmd Proposed)`, content: `\`\`\`\n${currentTerminalContent.join('\n')}\n\`\`\`` }];
                 // scrollToBottom();
             } catch (err) {
                 console.error("AI Agent: Error reading terminal content when no commands proposed:", err);
                 messages = [...messages, { type: 'error', content: `Error reading terminal: ${err}` }];
                 scrollToBottom();
                 throw new Error("Failed to read terminal state for next step."); // Stop loop
             }
        } else {
             console.warn("AI Agent: Terminal not available for reading state for next step.");
             // Decide if we should stop or continue without terminal context
             // For now, let's stop if terminal is expected but missing.
             messages = [...messages, { type: 'error', content: `Terminal not available to read state.` }];
             scrollToBottom();
             throw new Error("Terminal not available for reading state for next step."); // Stop loop
        }

        // --- Gather recent conversation history for the next prompt ---
        const historyLimit = 50; // Include last N messages (user/ai)


        // Construct the next prompt, including AI's last response, history, and terminal state
        const nextTerminalContext = currentTerminalContent.slice(-10).join('\n');
        // Use imported function to generate the continuation prompt
        const nextPrompt = getContinuationPrompt(messages, historyLimit, nextTerminalContext);
        console.log(`AI Agent: Step ${step + 1} - Continuing loop. Constructed next prompt:\n---\n` + nextPrompt + "\n---");
        // Recursive call for the next step
        await runAIInteractionLoop(nextPrompt, step + 1);
    } else if (!taskCompleteMatch && !waitForUserMatch) {
        // If we reach here, it means the loop should stop for some reason
        // (e.g., max steps reached earlier, or an error occurred and was handled)
        // Ensure loading state is reset if the loop ends unexpectedly without task completion or waiting
        isLoading = false;
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
      }, 50); // Slightly longer timeout might help ensure rendering completes
  }

  // --- Command Confirmation Logic ---
  function promptForCommandConfirmation(commandsToConfirm: string[]): Promise<boolean> {
      return new Promise((resolve) => {
          // Store the resolver function
          confirmationResolve = resolve;

          // Add a confirmation message object to the messages array
          messages = [
              ...messages,
              {
                  type: 'confirmation',
                  content: 'The AI proposes running the following command(s). Do you want to proceed?',
                  commands: commandsToConfirm,
                  // Assign handlers directly here
                  onAccept: () => handleConfirmation(true),
                  onReject: () => handleConfirmation(false)
              }
          ];
          scrollToBottom(); // Scroll to show the confirmation prompt
      });
  }

  function handleConfirmation(accepted: boolean) {
      if (confirmationResolve) {
          // Remove the confirmation message from the array
          messages = messages.filter(msg => msg.type !== 'confirmation');
          // Resolve the promise
          confirmationResolve(accepted);
          confirmationResolve = null; // Clear the resolver
          scrollToBottom(); // Scroll after removing confirmation
      } else {
          console.error("AI Agent: Confirmation resolved without a pending promise.");
      }
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
        <div class="ai-message">
          {#if message.thinking}
            <details class="ai-thinking-details">
              <summary class="ai-thinking-summary">AI Thinking...</summary>
              <div class="ai-thinking-content">
                {@html message.thinking} <!-- Or just {message.thinking} if no HTML expected -->
              </div>
            </details>
          {/if}
          <b>AI:</b> {message.content}
        </div>
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
       {:else if message.type === 'debug'}
         <div class="ai-debug">
           <p class="ai-debug-title">{message.title ?? 'DEBUG:'}</p>
           <pre class="ai-debug-content">{@html message.content}</pre> <!-- Use pre for formatting -->
         </div>
      {:else if message.type === 'error'}
        <div class="ai-message ai-error"><b>System Error:</b> {message.content}</div>
      {:else if message.type === 'confirmation'}
        <div class="ai-confirmation">
          <p>{message.content}</p>
          {#if message.commands && message.commands.length > 0}
            <pre class="ai-confirmation-commands"><code>{message.commands.join('\n')}</code></pre>
          {/if}
          <div class="ai-confirmation-buttons">
            <button class="confirm-button accept" on:click={message.onAccept}>Accept</button>
            <button class="confirm-button reject" on:click={message.onReject}>Reject</button>
          </div>
        </div>
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

  /* Debug message styling */
   .ai-debug {
     background-color: #444; /* Darker gray */
     border: 1px dashed #777;
     padding: 0.5rem;
     margin-bottom: 1rem;
     border-radius: 0.25rem;
     font-size: 0.8em;
     opacity: 0.8;
   }
   .ai-debug-title {
     font-weight: bold;
     color: #ccc !important;
     margin-bottom: 0.25rem;
     text-shadow: none;
   }
   .ai-debug-content {
     white-space: pre-wrap; /* Preserve whitespace and wrap */
     word-wrap: break-word;
     color: #ddd !important;
     text-shadow: none;
     max-height: 200px; /* Limit height */
     overflow-y: auto; /* Add scroll if needed */
     /* Minimal scrollbar for debug */
     scrollbar-width: thin;
     scrollbar-color: #666 #444;
   }
    .ai-debug-content::-webkit-scrollbar { width: 5px; }
    .ai-debug-content::-webkit-scrollbar-track { background: #444; }
    .ai-debug-content::-webkit-scrollbar-thumb { background-color: #666; border-radius: 3px; }


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
    margin-top: auto; /* Pushes input to bottom */
    padding: 1rem;
    background-color: var(--color-panel-bg);
    flex-shrink: 0;
    border-top: 1px solid rgba(255, 255, 255, 0.1); /* Add subtle separator */
    position: relative; /* Needed if confirmation overlaps */
    z-index: 10; /* Ensure input is above scrolled content */
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

 /* Confirmation prompt styling */
 .ai-confirmation {
   background-color: #2d3748; /* Darker blue-gray */
   border: 1px solid var(--color-border-gray);
   padding: 1rem;
   margin-bottom: 1rem;
   border-radius: 0.375rem;
   box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.2);
 }
 .ai-confirmation p {
   margin-bottom: 0.75rem;
   color: #e2e8f0 !important; /* Lighter text for contrast */
   text-shadow: none;
 }
 .ai-confirmation-commands {
   background-color: var(--color-primary-black);
   padding: 0.5rem;
   border-radius: 0.25rem;
   margin-bottom: 1rem;
   max-height: 150px;
   overflow-y: auto;
   /* Minimal scrollbar */
   scrollbar-width: thin;
   scrollbar-color: #666 #444;
 }
 .ai-confirmation-commands::-webkit-scrollbar { width: 5px; }
 .ai-confirmation-commands::-webkit-scrollbar-track { background: #444; }
 .ai-confirmation-commands::-webkit-scrollbar-thumb { background-color: #666; border-radius: 3px; }

 .ai-confirmation-commands code {
   white-space: pre-wrap;
   word-wrap: break-word;
   color: var(--color-text-white) !important;
   font-size: 0.85em;
   text-shadow: none;
   display: block; /* Ensure code takes full width */
 }
 .ai-confirmation-buttons {
   display: flex;
   justify-content: flex-end; /* Align buttons to the right */
   gap: 0.5rem; /* Space between buttons */
 }
 .confirm-button {
   padding: 0.375rem 0.75rem;
   border-radius: 0.375rem;
   font-weight: 600;
   font-size: 0.875rem;
   border: none;
   cursor: pointer;
   transition: background-color 150ms ease-in-out;
   text-shadow: none;
 }
 .confirm-button.accept {
   background-color: #38a169; /* Green */
   color: white !important;
 }
 .confirm-button.accept:hover {
   background-color: #2f855a; /* Darker green */
 }
 .confirm-button.reject {
   background-color: #e53e3e; /* Red */
   color: white !important;
 }
 .confirm-button.reject:hover {
   background-color: #c53030; /* Darker red */
 }

  /* Add styles for thinking details */
  .ai-thinking-details {
    margin-bottom: 0.5rem; /* Space between thinking and main response */
    background-color: rgba(255, 255, 255, 0.05); /* Slightly different background */
    border: 1px solid var(--color-border-gray);
    border-radius: 0.25rem;
    padding: 0.25rem 0.5rem;
  }
  .ai-thinking-summary {
    cursor: pointer;
    font-style: italic;
    color: #a0aec0 !important; /* Lighter gray */
    font-size: 0.8em;
    text-shadow: none;
    outline: none; /* Remove focus outline on summary */
  }
  .ai-thinking-content {
    padding-top: 0.5rem;
    font-size: 0.85em;
    color: #cbd5e0 !important; /* Slightly lighter than main text */
    text-shadow: none;
    white-space: pre-wrap; /* Preserve formatting */
    word-wrap: break-word;
  }

  /* Adjust AI message padding if thinking is present */
  .ai-message > b { /* Target the "AI:" label */
    display: block; /* Ensure label is on its own line if thinking is above */
    /* margin-top: 0.5rem; */ /* Add space above label if thinking is present - commented out for now */
  }
  /* More specific selector if needed */
   .ai-message details + b {
     margin-top: 0.5rem;
   }
</style>
