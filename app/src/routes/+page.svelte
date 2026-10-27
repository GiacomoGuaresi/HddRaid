<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  let coreVersion = $state("");

  $effect(() => {
    invoke<string>("core_version").then((v) => (coreVersion = v));
  });
</script>

<main>
  <h1>HddRaid</h1>
  <p>Motore {coreVersion}</p>
</main>

<style>
  main {
    font-family: system-ui, sans-serif;
    text-align: center;
    padding-top: 20vh;
  }
</style>
