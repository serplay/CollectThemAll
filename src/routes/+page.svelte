<script lang="ts">
  import Background from '../components/Background.svelte';
  import GameLibrary from '../components/GameLibrary.svelte';
</script>

<Background />
<main>
  <GameLibrary />
</main>

<style>
  :global(*) {
    box-sizing: border-box;
  }
  
  :global(:root) {
    font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
    font-size: 16px;
    color: #f6f6f6;
  }

  :global(html), :global(body) {
    margin: 0;
    padding: 0;
    min-height: 100%;
    width: 100%;
    overflow-x: hidden;
  }

  main {
    position: relative;
    z-index: 1;
    min-height: 100vh;
    width: 100%;
  }
</style>