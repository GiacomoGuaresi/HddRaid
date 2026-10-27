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
  /** Gli altri dischi collegati (nuovi o di un'altra coppia) prendono il
   *  posto di quelli che mancano, nell'ordine. */
  const extras = $derived.by(() => {
    const slots: Partial<Record<Side, VolumeView>> = {};
    if (name !== "noDisks" && name !== "oneDisk") return slots;
    const free = (["A", "B"] as Side[]).filter((x) => !s.disks[x]);
    s.others.forEach((volume, i) => {
      if (free[i]) slots[free[i]] = volume;
    });
    return slots;
  });
  /** Il primo disco in più, che dà il titolo alla schermata. */
  const extra = $derived(extras.A ?? extras.B ?? null);

  type Target = { kind: "pair"; side: Side } | { kind: "volume"; volume: VolumeView };
  let selected = $state<Target | null>(null);

  // Demo nel browser: `&disk=A` apre subito le informazioni di un disco.
  $effect(() => {
    const side = inTauri ? null : new URLSearchParams(location.search).get("disk");
    if (side === "A" || side === "B") select(side);
  });

  function select(side: Side) {
    const volume = extras[side];
    if (s.disks[side]) selected = { kind: "pair", side };
    else if (volume) selected = { kind: "volume", volume };
  }

  function diskState(side: Side) {
    if (same) return "ok";
    if (s.disks[side]) return "on";
    const volume = extras[side];
    if (volume) return volume.other_pair ? "other" : "new";
    return "off";
  }
  function diskLabel(side: Side) {
    return extras[side]?.name ?? label(side);
  }

  const title = $derived.by(() => {
    if (name === "formatting" && s.busy?.kind === "formatting") {
      return t("status.formatting.title", { disk: s.busy.label });
    }
    if (extra?.other_pair) return t("status.otherPair.title", { disk: extra.name });
    if (extra) return t("status.fresh.title", { disk: extra.name });
    if (name === "oneDisk") return t("status.oneDisk.title", { present: label(present) });
    if (same) return t("status.inSync.title");
    return t(`status.${name}.title`);
  });
  const text = $derived.by(() => {
    if (extra && name === "noDisks") {
      return t(extra.other_pair ? "status.otherPair.text" : "status.fresh.noPair");
    }
    if (extra?.other_pair) return t("status.otherPair.replace", { disk: label(missing) });
    if (extra) return t("status.fresh.text", { disk: label(missing) });
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
      onselect={name === "connected" || name === "oneDisk" || extra ? select : undefined}
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
  {#if name === "connected" || name === "oneDisk" || extra}
    <p class="note hint">{t("status.clickHint")}</p>
  {/if}

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
