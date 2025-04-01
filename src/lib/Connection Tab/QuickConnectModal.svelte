<script lang="ts">
  import { connectAndRunCommand as connect } from './DummyConnectionAgent';
  import { v4 as uuidv4 } from 'uuid';

  // Define the expected props, including callbacks
  type ConnectionDetails = {
      hostname: string;
      port: number;
      username: string;
      authMethod: 'password' | 'key';
  };
  type NewConnectionPayload = {
      id: string;
      name: string;
      type: string;
      details: ConnectionDetails;
  };
  type $$Props = {
    onClose: () => void; // Callback for closing
    onNewConnection: (payload: NewConnectionPayload) => void; // Callback for new connection
  };

  // Get props using $props rune
  const { onClose, onNewConnection } = $props();

  // --- State for the form inputs ---
  // Use $state() for all variables bound to form inputs
  let connectionName = $state('');
  let hostname = $state('');
  let port = $state(22);
  let username = $state('');
  let authMethod = $state<'password' | 'key'>('password');
  let password = $state('');
  let privateKeyPath = $state('');

  function closeModal() {
    // Call the onClose callback prop directly
    onClose();
  }

  async function handleSubmit() {
    console.log('Form submitted in modal. Attempting to connect with:', {
      hostname,
      port,
      username,
      authMethod,
    });

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
      const connectionDetails = {
        hostname,
        port,
        username,
        authMethod,
        password,
        privateKeyPath,
      };
      const connectionResult = await connect('');
      console.log(connectionResult); // Log the connection result
      onNewConnection(payload);
      closeModal();
    } catch (error) {
      console.error("Connection failed:", error);
      // Handle connection error, maybe show an error message to the user
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

      <div class="modal-actions">
        <button type="button" onclick={closeModal} class="button-secondary">
          Cancel
        </button>
         <button type="submit" class="button-primary">
          Connect & Add
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
</style>