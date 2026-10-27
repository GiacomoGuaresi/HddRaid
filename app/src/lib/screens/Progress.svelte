<!-- Avanzamento di allineamento, controllo completo e riparazione. -->
<script lang="ts">
  import { api, app, label } from "$lib/app.svelte";
  import Button from "$lib/components/Button.svelte";
  import Disks from "$lib/components/Disks.svelte";
  import ProgressBar from "$lib/components/ProgressBar.svelte";
  import Screen from "$lib/components/Screen.svelte";
  import { duration, percent } from "$lib/format";
  import { t } from "$lib/i18n/index.svelte";

  const busy = $derived(app.state!.busy!);
  const progress = $derived("progress" in busy ? busy.progress : null);
  const value = $derived(
    progress
      ? progress.bytes_total > 0
        ? percent(progress.bytes_done, progress.bytes_total)
        : percent(progress.done, progress.total)
      : 0,
  );

  const title = $derived(
    busy.kind === "checking"
      ? t("check.running.title")
      : busy.kind === "repairing"
        ? t("check.repairing")
        : t("syncing.title"),
  );
  const text = $derived(busy.kind === "checking" ? t("check.running.text") : t("syncing.text"));
</script>

<Screen {title} {text}>
  {#snippet hero()}
    <Disks
      labelA={label("A")}
      labelB={label("B")}
      a="on"
      b="on"
      link={busy.kind === "syncing" ? "sync" : "check"}
    />
  {/snippet}

  {#if progress}
    <div class="progress">
      <div class="numbers">
        <strong>{value}%</strong>
        <span class="muted">{t("syncing.count", { done: progress.done, total: progress.total })}</span>
      </div>
      <ProgressBar {value} />
      <div class="numbers muted">
        <span class="current">{progress.current}</span>
        <span>
          {progress.eta !== null
            ? t("syncing.eta", { time: duration(progress.eta) })
            : t("syncing.etaUnknown")}
        </span>
      </div>
    </div>
  {/if}

  {#snippet actions()}
    {#if busy.kind === "checking"}
      <Button onclick={() => api().stopCheck()}>{t("check.running.stop")}</Button>
    {/if}
  {/snippet}
</Screen>

<style>
  .progress {
    display: grid;
    gap: 10px;
  }
  .numbers {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 16px;
    font-size: 14px;
  }
  .numbers strong {
    font-size: 22px;
  }
  .current {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
