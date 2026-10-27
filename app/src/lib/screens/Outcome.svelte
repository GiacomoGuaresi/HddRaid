<!-- Fine di un'operazione: dischi allineati, espulsi, oppure un errore. -->
<script lang="ts">
  import { api, app, label } from "$lib/app.svelte";
  import Button from "$lib/components/Button.svelte";
  import Disks from "$lib/components/Disks.svelte";
  import Screen from "$lib/components/Screen.svelte";
  import { Eject } from "$lib/components/icons";
  import { bytes } from "$lib/format";
  import { t } from "$lib/i18n/index.svelte";

  interface Props {
    name: "done" | "ejected" | "error";
  }

  let { name }: Props = $props();

  const s = $derived(app.state!);
  const outcome = $derived(s.outcome);
  const ejecting = $derived(s.busy?.kind === "ejecting");
  const both = $derived(Boolean(s.disks.A && s.disks.B));

  const errorText = $derived.by(() => {
    if (outcome?.kind !== "error") return "";
    const disk = outcome.disk ? label(outcome.disk) : t("error.aDisk");
    return t(`error.${outcome.code}`, { disk });
  });
</script>

{#if name === "done"}
  <Screen title={t("done.title")} tone="success">
    {#snippet hero()}
      <Disks labelA={label("A")} labelB={label("B")} a="ok" b="ok" link="ok" />
    {/snippet}
    {#if outcome?.kind === "synced"}
      <div class="lines">
        {#if outcome.copied > 0}
          <p>{t("done.copied", { count: outcome.copied, size: bytes(outcome.bytes) })}</p>
        {/if}
        {#if outcome.deleted > 0}
          <p>{t("done.deleted", { count: outcome.deleted })}</p>
        {/if}
        {#if outcome.skipped.length > 0}
          <p class="warning">{t("done.skipped", { count: outcome.skipped.length })}</p>
        {/if}
      </div>
    {/if}
    {#snippet actions()}
      {#if both || ejecting}
        <Button variant="primary" size="large" disabled={ejecting} onclick={() => api().eject()}>
          <Eject size={20} />
          {ejecting ? t("done.ejecting") : t("done.eject")}
        </Button>
      {/if}
      <Button variant="ghost" disabled={ejecting} onclick={() => api().dismiss()}>
        {t("common.close")}
      </Button>
    {/snippet}
  </Screen>
{:else if name === "ejected"}
  <Screen title={t("ejected.title")} text={t("ejected.text")} tone="success">
    {#snippet hero()}
      <Disks labelA={label("A")} labelB={label("B")} a="gone" b="gone" link="ok" />
    {/snippet}
    {#snippet actions()}
      <Button onclick={() => api().dismiss()}>{t("common.ok")}</Button>
    {/snippet}
  </Screen>
{:else}
  <Screen title={t("error.title")} text={errorText} tone="danger">
    {#snippet hero()}
      <Disks
        labelA={label("A")}
        labelB={label("B")}
        a={s.disks.A ? "warn" : "off"}
        b={s.disks.B ? "warn" : "off"}
        link="error"
      />
    {/snippet}
    {#if outcome?.kind === "error"}
      <details>
        <summary class="muted">{t("error.details")}</summary>
        <code>{outcome.message}</code>
      </details>
    {/if}
    {#snippet actions()}
      <Button variant="primary" size="large" onclick={() => api().dismiss()}>{t("common.ok")}</Button>
    {/snippet}
  </Screen>
{/if}

<style>
  .lines {
    display: grid;
    gap: 6px;
    text-align: center;
    font-size: 17px;
  }
  .warning {
    margin-top: 8px;
    padding: 10px 16px;
    border-radius: var(--radius);
    background: var(--warning-soft);
  }
  details {
    text-align: center;
    font-size: 14px;
  }
  summary {
    cursor: pointer;
  }
  code {
    display: block;
    margin-top: 8px;
    padding: 10px 14px;
    border-radius: var(--radius-small);
    background: var(--surface-2);
    text-align: left;
    user-select: text;
    word-break: break-word;
  }
</style>
