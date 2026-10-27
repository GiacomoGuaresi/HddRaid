<!-- Controllo completo: proposta e risultato (l'avanzamento è in Progress). -->
<script lang="ts">
  import { api, app, label, other } from "$lib/app.svelte";
  import Button from "$lib/components/Button.svelte";
  import Disks from "$lib/components/Disks.svelte";
  import Screen from "$lib/components/Screen.svelte";
  import { Wrench } from "$lib/components/icons";
  import { t } from "$lib/i18n/index.svelte";
  import type { Side } from "$lib/types";

  interface Props {
    name: "checkResult";
  }

  let { name }: Props = $props();

  const s = $derived(app.state!);
  const outcome = $derived(s.outcome);

  const repairsBySide = $derived.by(() => {
    const count: Record<Side, number> = { A: 0, B: 0 };
    if (outcome?.kind === "checked") for (const r of outcome.repairs) count[r.bad]++;
    return count;
  });
  const healthy = $derived(
    outcome?.kind === "checked" && outcome.repairs.length === 0 && outcome.lost.length === 0,
  );
</script>

{#if outcome?.kind === "repaired"}
  <Screen title={t("check.healthy")} text={t("check.repaired", { count: outcome.done })} tone="success">
    {#snippet hero()}
      <Disks labelA={label("A")} labelB={label("B")} a="ok" b="ok" link="ok" />
    {/snippet}
    {#snippet actions()}
      <Button variant="primary" size="large" onclick={() => api().dismiss()}>{t("common.ok")}</Button>
    {/snippet}
  </Screen>
{:else if outcome?.kind === "checked" && !outcome.completed}
  <Screen title={t("check.paused")}>
    {#snippet hero()}
      <Disks labelA={label("A")} labelB={label("B")} a={s.disks.A ? "on" : "off"} b={s.disks.B ? "on" : "off"} link="check" />
    {/snippet}
    {#snippet actions()}
      <Button variant="primary" size="large" onclick={() => api().dismiss()}>{t("common.ok")}</Button>
    {/snippet}
  </Screen>
{:else if outcome?.kind === "checked"}
  <Screen
    title={healthy ? t("check.healthy") : t("check.problems")}
    text={healthy ? t("check.ok", { count: outcome.ok }) : undefined}
    tone={healthy ? "success" : "neutral"}
  >
    {#snippet hero()}
      <Disks
        labelA={label("A")}
        labelB={label("B")}
        a={repairsBySide.A > 0 ? "warn" : "ok"}
        b={repairsBySide.B > 0 ? "warn" : "ok"}
        link={healthy ? "ok" : "none"}
      />
    {/snippet}

    {#each ["A", "B"] as const as side (side)}
      {#if repairsBySide[side] > 0}
        <p class="card warning">
          {t("check.repairs", { count: repairsBySide[side], disk: label(side), other: label(other(side)) })}
        </p>
      {/if}
    {/each}
    {#if outcome.lost.length > 0}
      <div class="card danger">
        <p>{t("check.lost", { count: outcome.lost.length })}</p>
        <ul>
          {#each outcome.lost as path (path)}<li>{path}</li>{/each}
        </ul>
      </div>
    {/if}

    {#snippet actions()}
      {#if outcome.repairs.length > 0}
        <Button variant="ghost" onclick={() => api().dismiss()}>{t("check.due.later")}</Button>
        <Button variant="primary" size="large" onclick={() => api().repair()}>
          <Wrench size={20} />
          {t("check.repair")}
        </Button>
      {:else}
        <Button variant="primary" size="large" onclick={() => api().dismiss()}>{t("common.ok")}</Button>
      {/if}
    {/snippet}
  </Screen>
{/if}

<style>
  .card {
    padding: 14px 18px;
    border-radius: var(--radius);
    margin-bottom: 10px;
  }
  .warning {
    background: var(--warning-soft);
  }
  .danger {
    background: var(--danger-soft);
  }
  ul {
    margin: 8px 0 0;
    padding-left: 20px;
    font-size: 14px;
    user-select: text;
    word-break: break-word;
  }
</style>
