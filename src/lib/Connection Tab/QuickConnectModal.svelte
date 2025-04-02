<script lang="ts">
  // Removed: import { connectOnly } from './DummyConnectionAgent';
  import { v4 as uuidv4 } from 'uuid';
  import { addConnection, type ConnectionDetails, type ActiveConnection } from './ConnectionStore';

  // Define the expected props, including callbacks
  type NewConnectionPayload = {
      id: string;
      name: string;
      type: string;
      details: ConnectionDetails;
      connection_id?: string; // Added to store the connection ID from backend
  };
  // Update type to expect string (connectionId) or null
  type TerminalConnectFn = (options: { hostname: string; port: number; username: string; password?: string }) => Promise<string | null>;

  type $$Props = {
    onClose: () => void; // Callback for closing
    onNewConnection: (payload: NewConnectionPayload) => void; // Callback for new connection
    terminalConnect: TerminalConnectFn; // Prop to receive the terminal's connect function
  };

  // Get props using $props rune
  const { onClose, onNewConnection, terminalConnect } = $props(); // Get the new prop

  // --- State for the form inputs ---
  // Use $state() for all variables bound to form inputs
  let connectionName = $state('');
  let hostname = $state('');
  let port = $state(22);
  let username = $state('');
  let authMethod = $state<'password' | 'key'>('password');
  let password = $state('');
  let privateKeyPath = $state('');
  
  // Add state for connection status and errors
  let connecting = $state(false);
  let connectionError = $state<string | null>(null);

  function closeModal() {
    // Call the onClose callback prop directly
    onClose();
  }

  async function handleSubmit(event: Event) {
    // Prevent default form submission
    event.preventDefault();
    
    // Clear any previous errors
    connectionError = null;
    connecting = true;
    
    console.log('Form submitted in modal. Attempting to connect with:', {
      hostname,
      port,
      username,
      authMethod,
    });

    // Client-side validation
    if (!hostname.trim()) {
      connectionError = "Hostname is required";
      connecting = false;
      return;
    }
    
    if (!username.trim()) {
      connectionError = "Username is required";
      connecting = false;
      return;
    }
    
    if (authMethod === 'password' && !password.trim()) {
      connectionError = "Password is required";
      connecting = false;
      return;
    }
    
    if (authMethod === 'key' && !privateKeyPath.trim()) {
      connectionError = "Private key path is required";
      connecting = false;
      return;
    }

    const payload: NewConnectionPayload = {
      id: uuidv4(),
      name: connectionName || `${username}@${hostname}`,
      type: 'SSH',
      details: {
        hostname,
        port,
        username,
        authMethod,
      }
    };

    try {
      // Create a proper connection details object to pass to the connection function
      const connectionDetails = {
        hostname,
        port,
        username,
        authMethod,
        password,
        privateKeyPath,
      };
      
      console.log('Sending connection details to agent for connect-only:', connectionDetails);
      
      // Use the terminal's connect function passed via prop
      const returnedConnectionId = await terminalConnect({
          hostname: connectionDetails.hostname,
          port: connectionDetails.port,
          username: connectionDetails.username,
          password: connectionDetails.password // Pass password if available
          // Note: Key-based auth needs implementation in main.rs and Terminal.svelte connect
      });
      console.log('Terminal connect result (connectionId):', returnedConnectionId);

      if (!returnedConnectionId) {
          // The terminal component itself should display detailed errors.
          // We just show a generic failure message here in the modal's context.
          throw new Error("Connection failed. Check terminal for details.");
      }

      // If successful, store the returned connectionId and proceed
      payload.connection_id = returnedConnectionId; // Store the ID in the payload for the callback
      
      // Add to the connections store
      // Add the connection configuration to the store.
      // Note: isActive might need to be managed differently now, perhaps based on terminal state.
      // For now, we add it as inactive, assuming the parent component handles activation/selection.
      addConnection({
        id: payload.id,
        name: payload.name,
        type: payload.type,
        details: { // Store the full details needed to reconnect
          hostname,
          port,
          username,
          authMethod,
          password: authMethod === 'password' ? password : undefined, // Store password only if method is password
         privateKeyPath: authMethod === 'key' ? privateKeyPath : undefined // Store key path only if method is key
       },
       connectionId: returnedConnectionId, // Use the ID returned by terminalConnect
       isActive: true // Mark as active since connection succeeded
     });

     // Call the callback to notify parent (e.g., QuickConnect)
     onNewConnection(payload); // Pass payload which now includes connection_id
     closeModal();
    } catch (error) {
      console.error("Connection failed:", error);
      // Set the error message to display in the UI
      connectionError = `${error}`;
    } finally {
      connecting = false;
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      closeModal();
    }
  }

</script>

<svelte:window on:keydown={handleKeydown}/>

<div class="modal-backdrop" onclick={closeModal}>
  <!-- Floating error message outside the modal -->
  {#if connectionError}
    <div class="floating-error-message">
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="20" height="20" fill="currentColor">
        <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-2h2v2zm0-4h-2V7h2v6z"/>
      </svg>
      <span>{connectionError}</span>
      <button class="dismiss-error" onclick={() => connectionError = null} aria-label="Dismiss error">
        &times;
      </button>
    </div>
  {/if}

  <div class="modal-content" role="dialog" aria-modal="true" aria-labelledby="ssh-modal-title" onclick={(e: Event) => e.stopPropagation()}>
    <button class="close-button" onclick={closeModal} aria-label="Close modal">&times;</button>

    <h2 id="ssh-modal-title">New SSH Connection</h2>

    <form onsubmit={handleSubmit} class="ssh-form">

      <div class="form-group">
        <label for="hostname">Hostname or IP Address</label>
        <input type="text" id="hostname" bind:value={hostname} required placeholder="user@example.com or 192.168.1.100">
      </div>

      <div class="form-group form-group-inline">
          <div class="form-subgroup">
              <label for="port">Port</label>
              <input type="number" id="port" bind:value={port} required min="1" max="65535">
          </div>
          <div class="form-subgroup">
              <label for="username">Username</label>
              <input type="text" id="username" bind:value={username} required placeholder="root or admin">
          </div>
      </div>


      <fieldset class="form-group">
        <legend>Authentication Method</legend>
        <div class="radio-group">
          <label>
            <input type="radio" bind:group={authMethod} value="password"> Password
          </label>
          <label>
            <input type="radio" bind:group={authMethod} value="key"> Private Key
          </label>
        </div>
      </fieldset>

      {#if authMethod === 'password'}
        <div class="form-group">
          <label for="password">Password</label>
          <input type="password" id="password" bind:value={password}>
        </div>
      {/if}

      {#if authMethod === 'key'}
        <div class="form-group">
          <label for="private-key">Private Key Path</label>
          <input type="text" id="private-key" bind:value={privateKeyPath} placeholder="/path/to/your/id_rsa">
          </div>
      {/if}

      <!-- Removed inline error message -->
      
      <div class="modal-actions">
        <button type="button" onclick={closeModal} class="button-secondary" disabled={connecting}>
          Cancel
        </button>
        <button type="submit" class="button-primary" disabled={connecting}>
          {#if connecting}
            <span class="spinner"></span>
            Connecting...
          {:else}
            Connect & Add
          {/if}
        </button>
      </div>
    </form>
  </div>
</div>

<style>
  /* Styles remain the same */
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: rgba(0, 0, 0, 0.8);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 1000;
  }

  .modal-content {
    background-color: #1a1a1a;
    color: #e0e0e0;
    padding: 1.5rem 2rem;
    border-radius: 6px;
    border: 1px solid #444;
    box-shadow: 0 5px 20px rgba(0, 0, 0, 0.6);
    min-width: 400px;
    max-width: 90%;
    max-height: 90vh;
    overflow-y: auto;
    position: relative;
  }

  .close-button {
    position: absolute;
    top: 8px;
    right: 12px;
    background: none;
    border: none;
    font-size: 1.8rem;
    line-height: 1;
    cursor: pointer;
    color: #888;
  }
  .close-button:hover {
    color: #fff;
  }

  .modal-content h2 {
    margin-top: 0;
    margin-bottom: 1.5rem;
    color: #ffffff;
    font-size: 1.3rem;
    text-align: center;
    border-bottom: 1px solid #444;
    padding-bottom: 0.8rem;
  }

  .ssh-form {
    display: flex;
    flex-direction: column;
    gap: 1.2rem;
  }

  .form-group {
    display: flex;
    flex-direction: column;
  }

  .form-group-inline {
   display: flex;
    flex-direction: row;
    gap: 1rem;
    align-items: flex-end;
  }

  .form-subgroup {
      flex: 1;
      display: flex;
      flex-direction: column;
  }

  .form-group label,
  .form-group legend {
    margin-bottom: 0.5rem;
    font-weight: 600;
    font-size: 0.9rem;
    color: #bbbbbb;
  }

  .form-group input[type="text"],
  .form-group input[type="number"],
  .form-group input[type="password"] {
    padding: 0.7rem 0.9rem;
    border: 1px solid #555;
    border-radius: 4px;
    font-size: 1rem;
    background-color: #2a2a2a;
    color: #e0e0e0;
    box-sizing: border-box;
    width: 100%;
  }

  .form-group input::placeholder {
      color: #777;
      opacity: 1;
  }
  .form-group input:-ms-input-placeholder {
      color: #777;
  }
  .form-group input::-ms-input-placeholder {
      color: #777;
  }


  .form-group input:focus {
      border-color: #ffffff;
      outline: none;
      box-shadow: 0 0 0 2px rgba(255, 255, 255, 0.2);
      background-color: #333;
  }

  fieldset.form-group {
    border: 1px solid #555;
    padding: 0.8rem 1rem;
    border-radius: 4px;
    margin-top: 0.5rem;
  }

  fieldset.form-group legend {
      padding: 0 0.5rem;
      margin-bottom: 0.6rem;
      font-size: 0.85rem;
      color: #cccccc;
      width: auto;
  }

  .radio-group {
      display: flex;
      gap: 1.5rem;
      padding-top: 0.3rem;
  }
  .radio-group label {
      font-weight: normal;
      font-size: 0.95rem;
      display: flex;
      align-items: center;
      gap: 0.5rem;
      color: #e0e0e0;
      margin-bottom: 0;
      cursor: pointer;
  }
   .radio-group input[type="radio"] {
       margin-top: 0;
       appearance: none;
       -webkit-appearance: none;
       background-color: #444;
       border: 1px solid #777;
       width: 16px;
       height: 16px;
       border-radius: 50%;
       cursor: pointer;
       position: relative;
       vertical-align: middle;
   }
   .radio-group input[type="radio"]:checked {
       background-color: #fff;
       border: 1px solid #fff;
   }
   .radio-group input[type="radio"]:checked::after {
       content: '';
       display: block;
       width: 8px;
       height: 8px;
       border-radius: 50%;
       background-color: #1a1a1a;
       position: absolute;
       top: 50%;
       left: 50%;
       transform: translate(-50%, -50%);
   }


   .modal-actions {
       margin-top: 1.5rem;
       padding-top: 1rem;
       border-top: 1px solid #444;
       display: flex;
       justify-content: flex-end;
       gap: 0.8rem;
   }

  .button-primary,
  .button-secondary {
    border: 1px solid transparent;
    padding: 10px 20px;
    text-align: center;
    text-decoration: none;
    display: inline-block;
    font-size: 1rem;
    font-weight: 600;
    border-radius: 4px;
    cursor: pointer;
    transition: background-color 0.2s ease, border-color 0.2s ease;
  }

  .button-primary {
    background-color: #ffffff;
    color: #1a1a1a;
    border-color: #ffffff;
  }
  .button-primary:hover {
    background-color: #e0e0e0;
    border-color: #e0e0e0;
  }

  .button-secondary {
    background-color: #555;
    color: #ffffff;
    border-color: #555;
  }
   .button-secondary:hover {
    background-color: #666;
    border-color: #666;
  }
  
  /* Floating error message styles */
  .floating-error-message {
    position: fixed;
    top: 20px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 2000;
    
    background-color: #430f0f;
    border: 1px solid #ff5252;
    color: #ffa0a0;
    padding: 12px 16px;
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
    
    display: flex;
    align-items: center;
    gap: 10px;
    max-width: 90%;
    width: auto;
    
    animation: slideInDown 0.3s ease-out;
  }
  
  @keyframes slideInDown {
    from {
      transform: translate(-50%, -30px);
      opacity: 0;
    }
    to {
      transform: translate(-50%, 0);
      opacity: 1;
    }
  }
  
  .floating-error-message svg {
    flex-shrink: 0;
  }
  
  .floating-error-message span {
    font-size: 0.9rem;
    line-height: 1.4;
    flex-grow: 1;
  }
  
  .dismiss-error {
    background: none;
    border: none;
    color: #ffa0a0;
    font-size: 1.2rem;
    cursor: pointer;
    padding: 0;
    margin-left: 8px;
  }
  
  .dismiss-error:hover {
    color: #ffffff;
  }
  
  /* Spinner for loading state */
  .spinner {
    display: inline-block;
    width: 16px;
    height: 16px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-radius: 50%;
    border-top-color: #fff;
    animation: spin 1s ease infinite;
    margin-right: 8px;
    vertical-align: middle;
  }
  
  /* Button disabled state */
  button:disabled {
    opacity: 0.7;
    cursor: not-allowed;
  }
</style>