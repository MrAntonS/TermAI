<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type Terminal from '../terminal/Terminal.svelte';
  // Import prompt generation functions
  import { getPromptAfterAcceptedCommand, getPromptAfterRejectedCommand, getContinuationPrompt, getInitialPrompt } from './prompts';
  import './AIAgent.css'; // Import the CSS file

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
  // Define a type for the confirmation result
  type ConfirmationResult = {
      accepted: boolean;
      reason?: string | null; // Reason is optional and only present on rejection
  };

  let messages: Message[] = [
    { type: 'system', content: 'AI chat interface initialized. Ready for input.' },
    // Example messages removed for brevity
  ];
  let isLoading = false; // Add loading state
  let pendingCommands: string[] = []; // Commands awaiting confirmation
  let showConfirmation = false; // Flag to show confirmation UI (Now managed via message type)
  let confirmationPromiseResolver: ((result: ConfirmationResult) => void) | null = null; // Updated resolver type
  let isWaitingForUser = false; // Flag to indicate AI is waiting for user input
  let cancelRequested = false; // Flag to indicate user requested cancellation
  
  const MAX_AI_STEPS = 5; // Safety limit for interaction loop
  async function sendMessage() {
    const userQuestion = question.trim();
    if (userQuestion === '' || isLoading) return; // Don't send empty or while processing

    // --- Handle Rejection via Message Input ---
    if (confirmationPromiseResolver) {
        console.log(`AI Agent: sendMessage called while confirmation active. User input: "${userQuestion}"`);
        // Remove the confirmation message visually
        messages = messages.filter(msg => msg.type !== 'confirmation');
        // Resolve the promise with rejection and the user's message as the reason
        console.log("AI Agent: Resolving confirmation as rejected with reason:", userQuestion);
        confirmationPromiseResolver({ accepted: false, reason: userQuestion });
        confirmationPromiseResolver = null; // Clear the resolver
        question = ''; // Clear the input field
        scrollToBottom(); // Update UI
        // Stop further execution in sendMessage as this input was a rejection reason
        console.log("AI Agent: Rejection processed, returning from sendMessage.");
        return;
    }
    // Add a log to confirm when normal flow proceeds
    console.log("AI Agent: sendMessage proceeding with normal message flow (confirmation not active).");

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
    cancelRequested = false; // Reset cancellation flag for new request
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
        isLoading=false;
    }

    console.log(`AI Agent: Step ${step + 1} - Sending prompt:`, prompt);
    let rawResponse: string | null = null;
    try {
        rawResponse = await invoke<string>('send_to_gemini', { prompt: prompt });
    } catch (error) {
        // Handle potential errors during invoke itself (e.g., network issues, backend error)
        if (!cancelRequested) { // Only show error if not cancelled
            console.error(`AI Agent: Step ${step + 1} - Error invoking send_to_gemini:`, error);
            const invokeErrorMsg = typeof error === 'string' ? error : 'Failed to communicate with the AI backend.';
            messages = [...messages, { type: 'error', content: `Backend Error: ${invokeErrorMsg}` }];
            isLoading = false;
            scrollToBottom();
        } else {
            console.log(`AI Agent: Step ${step + 1} - Invoke failed after cancellation request.`);
            // Already handled cancellation flow below
        }
        // Whether cancelled or not, if invoke fails, we stop this loop iteration.
        // isLoading is reset in the cancel flow or the error flow.
        return; // Stop processing this step
    }

    // --- Check for Cancellation ---
    if (cancelRequested) {
        console.log(`AI Agent: Step ${step + 1} - Cancellation requested. Discarding response.`);
        messages = [...messages, { type: 'system', content: 'AI request cancelled by user.' }];
        isLoading = false;
        cancelRequested = false; // Reset flag
        scrollToBottom();
        return; // Stop the loop iteration
    }

    // Ensure rawResponse is not null before proceeding (shouldn't be if no error/cancellation)
    if (rawResponse === null) {
         console.error(`AI Agent: Step ${step + 1} - Raw response is unexpectedly null after invoke.`);
         messages = [...messages, { type: 'error', content: 'Internal error: Received null response.' }];
         isLoading = false;
         scrollToBottom();
         return; // Stop loop
    }

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
            const confirmationResult = await promptForCommandConfirmation(extractedCommands);

            if (confirmationResult.accepted) {
                // --- User Accepted ---
                isLoading = true; // Set loading before execution
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
                isLoading = true; // Set loading before continuing loop
                messages = [...messages, { type: 'system', content: `User rejected the proposed command(s). ${confirmationResult.reason ? `Reason: ${confirmationResult.reason}` : ''}` }];
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
                const nextPrompt = getPromptAfterRejectedCommand(nextTerminalContext, confirmationResult.reason); // Pass the reason
                console.log(`AI Agent: Step ${step + 1} - Constructed next prompt (after rejected commands, reason: ${confirmationResult.reason}):\n---\n` + nextPrompt + "\n---");
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
  function promptForCommandConfirmation(commandsToConfirm: string[]): Promise<ConfirmationResult> {
      return new Promise((resolve) => {
          // Store the resolver function
          confirmationPromiseResolver = resolve; // Use the new resolver name
          isLoading = false; // Enable input while waiting for confirmation

          // Add a confirmation message object to the messages array
          messages = [
              ...messages,
              {
                  type: 'confirmation',
                  content: 'The AI proposes running the following command(s). Do you want to proceed?',
                  commands: commandsToConfirm,
                  // Assign handlers directly here
                  onAccept: handleAcceptance, // Call new handler
                  onReject: rejectConfirmation // Call simplified rejection handler
              }
          ];
          scrollToBottom(); // Scroll to show the confirmation prompt
      });
  }

  // Separate handler for acceptance
  function handleAcceptance() {
      if (confirmationPromiseResolver) {
          // Remove the confirmation message from the array
          messages = messages.filter(msg => msg.type !== 'confirmation');
          // Resolve the promise with accepted status
          confirmationPromiseResolver({ accepted: true });
          confirmationPromiseResolver = null; // Clear the resolver
          scrollToBottom(); // Scroll after removing confirmation
      } else {
          console.error("AI Agent: Acceptance handled without a pending promise.");
      }
  }

  // Simplified handler for rejecting via the button (no reason prompted)
  function rejectConfirmation() {
      if (confirmationPromiseResolver) {
          // Remove the confirmation message from the array
          messages = messages.filter(msg => msg.type !== 'confirmation');
          // Resolve the promise with rejection and no reason
          confirmationPromiseResolver({ accepted: false, reason: null });
          confirmationPromiseResolver = null; // Clear the resolver
          scrollToBottom(); // Scroll after removing confirmation
      } else {
          console.error("AI Agent: Button rejection handled without a pending promise.");
      }
  }
  
  // --- Cancel Request ---
  function cancelRequest() {
      if (isLoading) {
          console.log("AI Agent: User requested cancellation.");
          cancelRequested = true;
          // Optionally update UI immediately, though the check in runAIInteractionLoop handles the logic
          // isLoading = false; // Let the loop handle this on return/check
      }
  }

  // --- Start New Task ---
  function startNewTask() {
      console.log("AI Agent: Starting new task, clearing conversation.");
      // Reset messages to initial state
      messages = [{ type: 'system', content: 'New task started. Chat history cleared.' }];
      question = ''; // Clear input field
      isLoading = false; // Ensure not in loading state
      isWaitingForUser = false; // Ensure not waiting for user
      cancelRequested = false; // Reset cancellation flag

      // If AI was waiting for command confirmation, implicitly cancel it
      if (confirmationPromiseResolver) {
          confirmationPromiseResolver({ accepted: false, reason: "Task restarted by user." }); // Resolve as rejected
          confirmationPromiseResolver = null;
      }

      // Optional: Focus the input textarea
      // aiTextareaElement?.focus();

      scrollToBottom(); // Scroll to show the cleared state
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
    <div class="ai-button-container">
        <button class="ai-new-task-button" title="Start New Task (Clears Chat)" on:click={startNewTask} disabled={isLoading}>
          New Task
        </button>
        {#if isLoading}
             <button class="ai-cancel-button" on:click={cancelRequest}>
               Cancel
             </button>
        {/if}
        <button class="ai-send-button" on:click={sendMessage} disabled={isLoading}>
          {#if isLoading}
            <span>Sending...</span>
          {:else}
            <span>Send</span>
          {/if}
        </button>
    </div>
  </div>
</section>
