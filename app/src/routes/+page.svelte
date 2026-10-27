<script lang="ts">
  import { app, screen, start } from "$lib/app.svelte";
  import { inTauri } from "$lib/api";
  import { ArrowLeft, Settings } from "$lib/components/icons";
  import { t } from "$lib/i18n/index.svelte";
  import { SCENARIO_NAMES } from "$lib/mock";
  import Status from "$lib/screens/Status.svelte";

  const current = $derived(screen());
  const inSettings = $derived(current === "settings" || current === "history");

  $effect(() => {
    start();
  });

  function pickDemo(e: Event) {
    const value = (e.target as HTMLSelectElement).value;
    location.search = `?demo=${value}`;
  }
</script>

<div class="app">
  <header>
    <span class="brand">HddRaid</span>
    {#if !inTauri}
      <select class="demo" onchange={pickDemo} aria-label="Demo">
        {#each SCENARIO_NAMES as name (name)}
          <option selected={location.search === `?demo=${name}`}>{name}</option>
        {/each}
      </select>
    {/if}
    {#if inSettings}
      <button class="icon-button" onclick={() => (app.view = "home")} title={t("common.back")}>
        <ArrowLeft size={22} />
      </button>
    {:else if current !== "loading" && current !== "welcome"}
      <button class="icon-button" onclick={() => (app.view = "settings")} title={t("common.settings")}>
        <Settings size={22} />
      </button>
    {/if}
  </header>

  {#key current}
    {#if current === "noDisks" || current === "oneDisk" || current === "comparing" || current === "inSync"}
      <Status name={current} />
    {/if}
  {/key}
</div>

<style>
  .app {
    height: 100vh;
    display: flex;
    flex-direction: column;
  }
  header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 20px 0;
  }
  .brand {
    flex: 1;
    font-weight: 700;
    color: var(--text-muted);
    letter-spacing: 0.01em;
  }
  .demo {
    font: inherit;
    font-size: 13px;
    padding: 4px 8px;
    border-radius: var(--radius-small);
    border: 1px dashed var(--border-strong);
    background: var(--surface);
    color: var(--text-muted);
  }
  .icon-button {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border: none;
    border-radius: 50%;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
  }
  .icon-button:hover {
    background: var(--surface-2);
    color: var(--text);
  }
</style>
