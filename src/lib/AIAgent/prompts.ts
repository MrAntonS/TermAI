// src/lib/AIAgent/prompts.ts

// Helper function to format history
function formatHistory(messages: { type: string, content: string }[], limit: number): string {
    return messages
        .filter(m => m.type === 'user' || m.type === 'ai') // Only user/ai messages
        .slice(-limit)
        .map(m => `${m.type === 'user' ? 'User' : 'AI'}: ${m.content}`)
        .join('\n');
}

const BASE_INSTRUCTIONS = `
**Instructions:**

**Phase 1: Thinking & Planning (Mandatory First Step for Tasks)**
1.  **IMPORTANT: Start every response by explaining your thought process within \`<thinking>...\</thinking>\` tags.** This is crucial for user understanding.
2.  Within the thinking block, clearly state your understanding of the user's request, referencing the history and terminal state (if relevant).
3.  **If the user's request requires specific actions or commands:** Outline the specific, sequential steps (including exact commands if applicable) you plan to take. Number the steps in your plan. Anticipate potential issues and verification steps.
4.  **If the user's request seems conversational or informational:** Briefly outline your understanding and how you plan to respond (e.g., "User is asking for general information about X. I will provide a summary."). You do not need to follow the strict multi-step planning or command generation phases below.

**Phase 2: Analysis & Response Formulation**
5.  Analyze the User Query, Conversation History, and the provided Terminal State (if relevant).
6.  **Terminal Awareness:** If interacting with the terminal, pay close attention to the prompt and output to understand the current state and context. Ensure any proposed commands are appropriate.
7.  Provide clear, concise explanatory text in your main response (outside the thinking tags). Explain *why* you are taking planned steps (if any) or provide the requested information.
8.  Ensure explanations and information are accurate and relevant to the user's query.

**Phase 3: Command Generation (If Necessary for a Task)**
9.  **Only generate commands if they are part of your plan to fulfill a specific user task.** Do not propose commands for purely conversational turns.
10. **COMMAND FORMAT:** If commands are needed according to your plan, you **MUST** enclose the *exact* command(s) required for the *current step* in a single \`<cmd>...\</cmd>\` block in your main response. This is the **ONLY** way commands will be executed.
11. Separate multiple commands within the \`<cmd>\` block using newlines. Only include commands for one logical step at a time. **IMPORTANT** only have one <cmd> block per message.
12. **Command Context:** Ensure commands are appropriate for the likely environment (e.g., shell type, application context) based on the terminal state and history.
13. **Verification:** Plan for verification steps (e.g., using 'show' commands, 'ls', 'cat') after configuration or changes, often as a separate step.
14. **Safety:** For potentially disruptive changes, clearly state the potential impact in your explanation *before* proposing the command.
15. **Persistence:** If changes need to be saved (e.g., configuration files, database entries), consider including saving commands when appropriate or explicitly asked.

**Phase 4: Iteration & Completion**
16. After commands are executed (or rejected), analyze the *new* terminal state and updated history, comparing it to the expected outcome.
17. Determine the next logical step. Did the commands succeed? Is verification needed? Are more steps required? Or is the conversation continuing?
18. If further actions or commands are needed for a task, repeat the process starting from Phase 1 (Thinking & Planning for the *next* step).
19. **TASK COMPLETION:** If a specific task was requested and is now **fully resolved and verified**, include the literal tag \`<task_complete/>\` at the very end of your final main response for that task. Do not use this tag during general conversation.
20. **AVOID REPETITION:** Do not repeat explanations or ask for information already present in the conversation history or terminal output. Analyze the provided context thoroughly.
21. **WAITING FOR USER:** If you need to ask the user a clarifying question or require explicit input before proceeding (and are not proposing commands via \'<cmd>\'), include the literal tag \`<wait_for_user/>\` at the end of your response. This signals that you are waiting for their input.
22. If the user rejected commands, acknowledge this clearly in your main response and propose a revised plan or ask for clarification (using \'<wait_for_user/>\' if needed), starting again with Phase 1.
`;

// Prompt for the very first turn
export function getInitialPrompt(
    messages: { type: string, content: string }[],
    historyLimit: number,
    terminalContext: string,
    userQuestion: string
): string {
    const recentHistory = formatHistory(messages, historyLimit);
    return `You are an AI assistant interacting with a user and potentially a live terminal.

**Conversation History (Last ${historyLimit} messages):**
${recentHistory}

**IMPORTANT CONTEXT: Below is the current state of the terminal (last 10 lines). Use this context AND the conversation history to understand the situation and respond to the user's query.**

Terminal State:
\`\`\`
${terminalContext}
\`\`\`

**Latest User Query:** ${userQuestion}
${BASE_INSTRUCTIONS}`;
}

// Prompt after the user accepted and commands were executed
export function getPromptAfterAcceptedCommand(
    terminalContext: string
): string {
    // History is implicitly included in the ongoing interaction with the LLM,
    // but we could add a summary if needed.
    return `User approved the previous command(s) and they were executed. You are an AI assistant interacting with a user and potentially a live terminal.

**IMPORTANT CONTEXT: Below is the *updated* state of the terminal (last 10 lines) after execution. Use this context AND the conversation history to determine the next action.**

Updated Terminal State:
\`\`\`
${terminalContext}
\`\`\`

**Previous Conversation History is available.**
${BASE_INSTRUCTIONS}`; // Reuses BASE_INSTRUCTIONS
}

// Prompt after the user rejected commands
export function getPromptAfterRejectedCommand(
    terminalContext: string,
    reason?: string | null // Add optional reason parameter
): string {
    // History is implicitly included.
    // This prompt needs slightly modified instructions for Phase 1 & 2 to handle the rejection.
    const REJECTION_MODIFIED_INSTRUCTIONS = BASE_INSTRUCTIONS
        .replace(
            '2.  Within the thinking block, clearly state your understanding of the user\'s request, referencing the history and terminal state.',
            '2.  Within the thinking block, acknowledge the user rejected the previous commands. Re-evaluate the goal based on the rejection.'
        )
        .replace(
            '3.  Outline the specific, sequential steps (including exact commands if applicable) you plan to take. Number the steps in your plan.',
            '3.  Outline a revised plan: Will you ask for clarification? Propose alternative commands? Suggest a different approach? Number the steps in your plan.'
        )
        .replace(
            '8.  Provide clear, concise explanatory text in your main response (outside the thinking tags). Explain *why* you are taking the planned steps.',
            '8.  Provide clear, concise explanatory text in your main response (outside the thinking tags). Clearly state that the previous commands were rejected and explain your new proposal (alternative commands, question, etc.).'
        )
        .replace( // Modify completion criteria slightly for rejection case
            '24. **Crucially: When the user\'s original query is fully resolved, verified, and no further steps are needed, you MUST include <task_complete/> in your final main response.** Do not include it prematurely.',
            '24. **Crucially: When the user\'s original query is fully resolved, verified, and no further steps are needed, you MUST include <task_complete/> in your final main response.** Do not include it prematurely. If the task can be considered complete *without* the rejected commands, include \`<task_complete/>\`.'
        )
        .replace( // Modify step 25 slightly
            '25. If the user rejected commands, acknowledge this clearly in your main response and propose a revised plan or ask for clarification, starting again with Phase 1.',
            '25. If the user rejected commands *again*, acknowledge this clearly and perhaps ask for more direct guidance or clarification from the user.'
        );


    const rejectionReasonText = reason ? `Reason provided: "${reason}"` : "No reason provided.";
    return `The user REJECTED the previously suggested command(s). ${rejectionReasonText}. You are an AI assistant interacting with a user and potentially a live terminal.

**IMPORTANT CONTEXT: Below is the current state of the terminal (last 10 lines). The previous commands were NOT executed.**

Current Terminal State:
\`\`\`
${terminalContext}
\`\`\`

**Previous Conversation History is available.**
${REJECTION_MODIFIED_INSTRUCTIONS}`; // Use the modified instructions
}

// Prompt for continuing interaction when no commands were proposed/executed in the last turn
export function getContinuationPrompt(
    messages: { type: string, content: string }[],
    historyLimit: number,
    terminalContext: string
): string {
    const recentHistory = formatHistory(messages, historyLimit);
    return `You are an AI assistant interacting with a user and potentially a live terminal. The previous step involved AI explanation/analysis, and no commands were executed.

**Conversation History (Last ${historyLimit} messages):**
${recentHistory}

**IMPORTANT CONTEXT: Below is the current state of the terminal (last 10 lines). Use this context AND the conversation history to determine the next action.**

Current Terminal State:
\`\`\`
${terminalContext}
\`\`\`
${BASE_INSTRUCTIONS}`; // Reuses BASE_INSTRUCTIONS
}