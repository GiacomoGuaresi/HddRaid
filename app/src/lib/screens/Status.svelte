<!-- Stato dei dischi: nessuno, uno, confronto in corso, entrambi collegati. -->
<script lang="ts">
  import { inTauri } from "$lib/api";
  import { api, app, label } from "$lib/app.svelte";
  import Button from "$lib/components/Button.svelte";
  import DiskPopup from "$lib/components/DiskPopup.svelte";
  import Disks from "$lib/components/Disks.svelte";
  import Screen from "$lib/components/Screen.svelte";
  import { Eject, HeartPulse, RefreshCw } from "$lib/components/icons";
  import { relative } from "$lib/format";
  import { t } from "$lib/i18n/index.svelte";
  import type { Side, VolumeView } from "$lib/types";

  interface Props {
    name: "noDisks" | "oneDisk" | "comparing" | "connected" | "formatting";
  }

  let { name }: Props = $props();

  const s = $derived(app.state!);
  const present = $derived(s.disks.A ? "A" : "B");
  const missing = $derived(present === "A" ? "B" : "A");
  /** Confrontati e senza niente da fare. */
  const same = $derived(name === "connected" && s.compared);
  /** Un disco nuovo collegato: si mostra al posto di quello che manca. */
  const fresh = $derived(
    name === "oneDisk" ? (s.others.find((o) => !o.other_pair) ?? null) : null,
  );
  const foreign = $derived(s.others.filter((o) => o.other_pair));

  type Target = { kind: "pair"; side: Side } | { kind: "volume"; volume: VolumeView };
  let selected = $state<Target | null>(null);

  // Demo nel browser: `&disk=A` apre subito le informazioni di un disco.
  $effect(() => {
    const side = inTauri ? null : new URLSearchParams(location.search).get("disk");
    if (side === "A" || side === "B") select(side);
  });

  function select(side: Side) {
    if (s.disks[side]) selected = { kind: "pair", side };
    else if (fresh) selected = { kind: "volume", volume: fresh };
  }

  function diskState(side: Side) {
    if (same) return "ok";
    if (s.disks[side]) return "on";
    return fresh ? "new" : "off";
  }
  function diskLabel(side: Side) {
    return !s.disks[side] && fresh ? fresh.name : label(side);
  }

  const title = $derived.by(() => {
    if (name === "formatting" && s.busy?.kind === "formatting") {
      return t("status.formatting.title", { disk: s.busy.label });
    }
    if (name === "oneDisk" && fresh) return t("status.fresh.title", { disk: fresh.name });
    if (name === "oneDisk") return t("status.oneDisk.title", { present: label(present) });
    if (same) return t("status.inSync.title");
    return t(`status.${name}.title`);
  });
  const text = $derived.by(() => {
    if (name === "oneDisk" && fresh) return t("status.fresh.text", { disk: label(missing) });
    if (name === "oneDisk") {
      return t("status.oneDisk.text", { present: label(present), missing: label(missing) });
    }
    if (same) return t("status.inSync.text");
    return t(`status.${name}.text`);
  });
</script>

<Screen {title} {text} tone={same ? "success" : "neutral"}>
  {#snippet hero()}
    <Disks
      labelA={diskLabel("A")}
      labelB={diskLabel("B")}
      a={diskState("A")}
      b={diskState("B")}
      link={same ? "ok" : name === "comparing" || name === "formatting" ? "compare" : "none"}
      onselect={name === "connected" || name === "oneDisk" ? select : undefined}
    />
  {/snippet}

  {#if name === "connected"}
    {#if s.check_due}
      <p class="note warning">{t("status.connected.checkDue")}</p>
    {/if}
    <p class="note">
      {#if s.last_sync}{t("status.inSync.lastSync", { when: relative(s.last_sync) })}{/if}
      {#if s.last_sync && s.last_check}&nbsp;·&nbsp;{/if}
      {#if s.last_check}{t("check.lastCheck", { when: relative(s.last_check) })}{/if}
    </p>
  {/if}
  {#if name === "connected" || name === "oneDisk"}
    <p class="note hint">{t("status.clickHint")}</p>
  {/if}
  {#each foreign as volume (volume.root)}
    <p class="note warning">{t("status.foreign", { name: volume.name })}</p>
  {/each}

  {#snippet actions()}
    {#if name === "connected"}
      <Button variant="ghost" onclick={() => api().eject()}>
        <Eject size={18} />
        {t("done.eject")}
      </Button>
      <Button variant={s.check_due ? "primary" : "secondary"} size="large" onclick={() => api().startCheck()}>
        <HeartPulse size={20} />
        {t("status.connected.check")}
      </Button>
      <Button variant="primary" size="large" onclick={() => api().compare()}>
        <RefreshCw size={20} strokeWidth={2.5} />
        {t("summary.sync")}
      </Button>
    {/if}
  {/snippet}
</Screen>

{#if selected}
  <DiskPopup target={selected} onclose={() => (selected = null)} />
{/if}

<style>
  .note {
    text-align: center;
    color: var(--text-muted);
  }
  .note + .note {
    margin-top: 10px;
  }
  .hint {
    font-size: 14px;
    color: var(--text-faint);
  }
  .warning {
    padding: 10px 16px;
    border-radius: var(--radius);
    background: var(--warning-soft);
    color: var(--text);
  }
</style>
