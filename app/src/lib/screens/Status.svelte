<!-- Schermate di attesa: nessun disco, un disco, confronto, dischi uguali. -->
<script lang="ts">
  import { app, label } from "$lib/app.svelte";
  import Disks from "$lib/components/Disks.svelte";
  import Screen from "$lib/components/Screen.svelte";
  import { relative } from "$lib/format";
  import { t } from "$lib/i18n/index.svelte";

  interface Props {
    name: "noDisks" | "oneDisk" | "comparing" | "inSync";
  }

  let { name }: Props = $props();

  const s = $derived(app.state!);
  const present = $derived(s.disks.A ? "A" : "B");
  const missing = $derived(present === "A" ? "B" : "A");

  const title = $derived.by(() => {
    switch (name) {
      case "oneDisk":
        return t("status.oneDisk.title", { present: label(present) });
      default:
        return t(`status.${name}.title`);
    }
  });
  const text = $derived.by(() => {
    switch (name) {
      case "oneDisk":
        return t("status.oneDisk.text", { present: label(present), missing: label(missing) });
      default:
        return t(`status.${name}.text`);
    }
  });
</script>

<Screen {title} {text} tone={name === "inSync" ? "success" : "neutral"}>
  {#snippet hero()}
    <Disks
      labelA={label("A")}
      labelB={label("B")}
      a={name === "inSync" ? "ok" : s.disks.A ? "on" : "off"}
      b={name === "inSync" ? "ok" : s.disks.B ? "on" : "off"}
      link={name === "inSync" ? "ok" : name === "comparing" ? "compare" : "none"}
    />
  {/snippet}

  {#if name === "inSync" && s.last_sync}
    <p class="note">{t("status.inSync.lastSync", { when: relative(s.last_sync) })}</p>
  {/if}
  {#each s.foreign as foreign (foreign)}
    <p class="note warning">{t("status.foreign", { name: foreign })}</p>
  {/each}
</Screen>

<style>
  .note {
    text-align: center;
    color: var(--text-muted);
  }
  .note + .note {
    margin-top: 8px;
  }
  .warning {
    padding: 10px 16px;
    border-radius: var(--radius);
    background: var(--warning-soft);
    color: var(--text);
  }
</style>
