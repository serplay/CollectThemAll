<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';

  let { children } = $props();

  // The overlay window loads the SPA entry (index.html); detect it by window label and
  // client-route to /overlay. Reading the label is synchronous, so the main window renders
  // immediately and the overlay window shows a blank panel (not the library) until redirected.
  let isOverlayWindow = false;
  try {
    // @ts-expect-error — __TAURI_INTERNALS__ is injected by Tauri at runtime
    isOverlayWindow = window.__TAURI_INTERNALS__?.metadata?.currentWindow?.label === 'overlay';
  } catch {
    /* not running under Tauri */
  }

  let ready = $state(!isOverlayWindow);

  onMount(async () => {
    if (isOverlayWindow && !location.pathname.replace(/\/$/, '').endsWith('/overlay')) {
      await goto('/overlay');
    }
    ready = true;
  });
</script>

{#if ready}
  {@render children()}
{:else}
  <div class="overlay-boot"></div>
{/if}

<style>
  .overlay-boot {
    height: 100vh;
    width: 100%;
    background: #0e0b1c;
  }
</style>
