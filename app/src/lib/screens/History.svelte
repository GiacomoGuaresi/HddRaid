<!-- Storico: cosa è successo e quando. I file copiati uno per uno restano
     nell'indice; qui si vedono gli allineamenti, le cancellazioni e i controlli. -->
<script lang="ts">
  import { api } from "$lib/app.svelte";
  import { dateTime } from "$lib/format";
  import { t } from "$lib/i18n/index.svelte";
  import type { HistoryEvent } from "$lib/types";

  const SHOWN = new Set([
    "synced",
    "deleted",
    "folder_deleted",
    "repaired",
    "created",
    "index_rebuilt",
    "full_check_completed",
    "skipped",
  ]);

  let events = $state<HistoryEvent[] | null>(null);

  $effect(() => {
    api()
      .history()
      .then((all) => (events = all.filter((e) => SHOWN.has(e.kind))));
  });

  /** Valori di `details`, come "copied=3 deleted=1". */
  function details(e: HistoryEvent): Record<string, number> {
    return Object.fromEntries(
      (e.details ?? "")
        .split(" ")
        .map((part) => part.split("="))
        .filter(([, v]) => v !== undefined && !Number.isNaN(Number(v)))
        .map(([k, v]) => [k, Number(v)]),
    );
  }

  function describe(e: HistoryEvent): string {
    const d = details(e);
    if (e.kind === "synced") {
      const parts = [t("history.synced", { count: d.copied ?? 0 })];
      if (d.deleted) parts.push(t("history.deletedCount", { count: d.deleted }));
      return parts.join(" · ");
    }
    if (e.kind === "full_check_completed" && d.ok !== undefined) {
      return t("check.ok", { count: d.ok });
    }
    const where = e.disk ? ` ${t("history.on", { disk: e.disk })}` : "";
    return e.path ? `${e.path}${where}` : where.trim();
  }
</script>

<section class="history">
  <h1>{t("history.title")}</h1>
  {#if events && events.length === 0}
    <p class="muted">{t("history.empty")}</p>
  {:else if events}
    <ol>
      {#each events as event, i (i)}
        <li class={event.kind}>
          <span class="dot"></span>
          <div>
            <strong>{t(`history.kinds.${event.kind}`)}</strong>
            <span class="muted">{describe(event)}</span>
          </div>
          <time class="muted">{dateTime(event.at)}</time>
        </li>
      {/each}
    </ol>
  {/if}
</section>

<style>
  .history {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 8px 40px 32px;
    max-width: 720px;
    width: 100%;
    margin: 0 auto;
  }
  h1 {
    margin-bottom: 20px;
  }
  ol {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    display: flex;
    align-items: baseline;
    gap: 14px;
    padding: 12px 0;
    border-bottom: 1px solid var(--border);
  }
  li > div {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  li .muted {
    font-size: 14px;
    word-break: break-word;
    user-select: text;
  }
  time {
    font-size: 13px;
    white-space: nowrap;
  }
  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--accent);
    flex: none;
    transform: translateY(-1px);
  }
  .synced .dot,
  .full_check_completed .dot,
  .repaired .dot {
    background: var(--success);
  }
  .deleted .dot,
  .folder_deleted .dot {
    background: var(--danger);
  }
  .skipped .dot {
    background: var(--warning);
  }
</style>
