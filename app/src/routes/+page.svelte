<script lang="ts">
  import { app, screen, start } from "$lib/app.svelte";
  import { inTauri } from "$lib/api";
  import { ArrowLeft } from "$lib/components/icons";
  import { t } from "$lib/i18n/index.svelte";
  import { SCENARIO_NAMES } from "$lib/mock";
  import { fade } from "svelte/transition";
  import Check from "$lib/screens/Check.svelte";
  import History from "$lib/screens/History.svelte";
  import Outcome from "$lib/screens/Outcome.svelte";
  import Progress from "$lib/screens/Progress.svelte";
  import Questions from "$lib/screens/Questions.svelte";
  import Status from "$lib/screens/Status.svelte";
  import Summary from "$lib/screens/Summary.svelte";

  const current = $derived(screen());
  // Con "riduci movimento" del sistema, le schermate cambiano senza dissolvenza.
  const still =
    typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches;
  const inSubview = $derived(current === "history" || current === "questions");

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
    <span class="spacer"></span>
    {#if !inTauri}
      <select class="demo" onchange={pickDemo} aria-label="Demo">
        {#each SCENARIO_NAMES as name (name)}
          <option selected={new URLSearchParams(location.search).get("demo") === name}>{name}</option>
        {/each}
      </select>
    {/if}
    {#if inSubview}
      <button
        class="icon-button"
        onclick={() => (app.view = "home")}
        title={t("common.back")}
      >
        <ArrowLeft size={22} />
      </button>
    {/if}
  </header>

  <main class="stage">
  {#key current}
    <div class="page" in:fade={{ duration: still ? 0 : 250, delay: still ? 0 : 120 }} out:fade={{ duration: still ? 0 : 120 }}>
    {#if current === "noDisks" || current === "oneDisk" || current === "comparing" || current === "connected" || current === "formatting"}
      <Status name={current} />
    {:else if current === "summary"}
      <Summary />
    {:else if current === "questions"}
      <Questions />
    {:else if current === "syncing" || current === "checking"}
      <Progress />
    {:else if current === "done" || current === "ejected" || current === "error"}
      <Outcome name={current} />
    {:else if current === "checkResult"}
      <Check name={current} />
    {:else if current === "history"}
      <History />
    {/if}
    </div>
  {/key}
  </main>
</div>

<style>
  .app {
    height: 100vh;
    display: flex;
    flex-direction: column;
  }
  /* Le schermate si sovrappongono durante il cambio, senza salti. */
  .stage {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template: minmax(0, 1fr) / minmax(0, 1fr);
  }
  .page {
    grid-area: 1 / 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 20px 0;
  }
  .spacer {
    flex: 1;
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
