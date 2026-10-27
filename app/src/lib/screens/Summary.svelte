<!-- Riepilogo di cosa farà l'allineamento, con le domande e il pulsante Allinea. -->
<script lang="ts">
  import { api, app, label, other } from "$lib/app.svelte";
  import Button from "$lib/components/Button.svelte";
  import Disks from "$lib/components/Disks.svelte";
  import Screen from "$lib/components/Screen.svelte";
  import {
    Check,
    ChevronDown,
    Copy,
    FolderPlus,
    MoveRight,
    PencilLine,
    RefreshCw,
    TriangleAlert,
  } from "$lib/components/icons";
  import { bytes } from "$lib/format";
  import { t } from "$lib/i18n/index.svelte";
  import type { Side, SummaryLine } from "$lib/types";

  const summary = $derived(app.state!.summary!);
  const questions = $derived(summary.questions);
  const answered = $derived(questions.filter((q) => app.answers[q.key]).length);
  const allAnswered = $derived(answered === questions.length);
  const short = $derived(
    (["A", "B"] as Side[]).find((s) => summary.missing_space[s] > 0) ?? null,
  );

  /** File da cancellare per disco, secondo le risposte. */
  const deletions = $derived.by(() => {
    const count: Record<Side, number> = { A: 0, B: 0 };
    for (const q of questions) {
      if (app.answers[q.key] === "delete" && q.deleted_on) count[other(q.deleted_on)]++;
    }
    return count;
  });

  /** "37 file da allineare · 1,2 GB" */
  const total = $derived.by(() => {
    const count = summary.lines.reduce((n, l) => n + l.count, 0);
    if (count === 0) return undefined;
    const size = summary.lines.reduce((n, l) => n + l.bytes, 0);
    const text = t("summary.total", { count });
    return size > 0 ? `${text} · ${bytes(size)}` : text;
  });

  let showFiles = $state(false);
  let confirming = $state(false);

  const icons = { new: Copy, changed: PencilLine, moved: MoveRight, folders: FolderPlus };

  function lineTitle(l: SummaryLine) {
    return t(`summary.${l.kind}.title`, { count: l.count, from: label(l.from) });
  }
  function lineAction(l: SummaryLine) {
    return t(`summary.${l.kind}.action`, { count: l.count, to: label(other(l.from)) });
  }

  function align() {
    if (deletions.A + deletions.B > 0 && !confirming) {
      confirming = true;
      return;
    }
    confirming = false;
    api().sync($state.snapshot(app.answers));
  }
</script>

<Screen title={t("summary.title")} text={total}>
  {#snippet hero()}
    <Disks
      labelA={label("A")}
      labelB={label("B")}
      a="on"
      b="on"
      compact
      link={questions.length > 0 && !allAnswered ? "question" : "none"}
    />
  {/snippet}

  {#if short}
    <p class="notice danger">
      <span class="notice-icon"><TriangleAlert size={20} /></span>
      {t("summary.noSpace", { disk: label(short), size: bytes(summary.missing_space[short]) })}
    </p>
  {/if}

  {#if questions.length > 0}
    <div class="notice questions" class:done={allAnswered}>
      <span class="notice-icon">
        {#if allAnswered}<Check size={20} strokeWidth={3} />{:else}<span class="mark">?</span>{/if}
      </span>
      <div class="notice-text">
        <strong>
          {allAnswered
            ? t("summary.questions.done")
            : t("summary.questions.title", { count: questions.length - answered })}
        </strong>
        {#if !allAnswered}<span class="muted">{t("summary.questions.text")}</span>{/if}
      </div>
      <Button variant={allAnswered ? "ghost" : "secondary"} onclick={() => (app.view = "questions")}>
        {allAnswered ? t("summary.questions.edit") : t("summary.questions.answer")}
      </Button>
    </div>
  {/if}

  {#if summary.lines.length === 0}
    <p class="only muted">{t("summary.onlyQuestions")}</p>
  {:else}
    <ul class="group">
      {#each summary.lines as line, i (line.kind + line.from)}
        {@const Icon = icons[line.kind]}
        <li style:--i={i}>
          <span class="icon {line.kind}"><Icon size={20} /></span>
          <div class="line-text">
            <strong>{lineTitle(line)}</strong>
            <span class="muted">{lineAction(line)}</span>
          </div>
          {#if line.bytes > 0}<span class="size">{bytes(line.bytes)}</span>{/if}
        </li>
      {/each}
    </ul>
  {/if}

  {#if summary.files.length > 0}
    <button class="toggle" onclick={() => (showFiles = !showFiles)}>
      <span class:open={showFiles}><ChevronDown size={18} /></span>
      {showFiles ? t("summary.hideFiles") : t("summary.showFiles")}
    </button>
    {#if showFiles}
      <ul class="files">
        {#each summary.files as file (file.kind + file.path)}
          <li>
            <span class="path">{file.path}</span>
            <span class="muted">{label(file.from)}</span>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}

  {#snippet actions()}
    <Button variant="primary" size="large" disabled={short !== null} onclick={align}>
      <RefreshCw size={20} strokeWidth={2.5} />
      {t("summary.sync")}
    </Button>
  {/snippet}
</Screen>

{#if confirming}
  <div class="backdrop" role="presentation" onclick={() => (confirming = false)}>
    <div
      class="dialog"
      role="alertdialog"
      aria-modal="true"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.key === "Escape" && (confirming = false)}
    >
      <span class="warn-icon"><TriangleAlert size={28} /></span>
      {#each ["A", "B"] as const as side (side)}
        {#if deletions[side] > 0}
          <h2>{t("questions.confirm.title", { count: deletions[side], disk: label(side) })}</h2>
        {/if}
      {/each}
      <p class="muted">{t("questions.confirm.text")}</p>
      <div class="dialog-actions">
        <Button onclick={() => (confirming = false)}>{t("questions.confirm.cancel")}</Button>
        <Button variant="danger" onclick={align}>{t("questions.confirm.ok")}</Button>
      </div>
    </div>
  </div>
{/if}

<style>
  .group {
    list-style: none;
    margin: 0;
    padding: 0;
    border-radius: var(--radius-large);
    background: var(--surface);
    border: 1px solid var(--border);
    box-shadow: var(--shadow);
    overflow: hidden;
  }
  .group li {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 20px;
    animation: card-in 0.45s calc(0.3s + var(--i) * 0.07s) var(--ease) both;
  }
  .group li + li {
    border-top: 1px solid var(--border);
  }
  .line-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .line-text .muted {
    font-size: 15px;
  }
  .size {
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    color: var(--text-muted);
    white-space: nowrap;
  }
  .icon {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: 12px;
    background: var(--accent-soft);
    color: var(--accent);
    flex: none;
  }
  .notice {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 16px 14px 14px;
    margin: 0 0 14px;
    border-radius: var(--radius-large);
  }
  .notice-icon {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: 50%;
    flex: none;
    color: #fff;
  }
  .notice-text {
    flex: 1;
    display: flex;
    flex-direction: column;
  }
  .notice-text .muted {
    font-size: 15px;
  }
  .mark {
    font-size: 22px;
    font-weight: 800;
    line-height: 1;
  }
  .questions {
    background: var(--warning-soft);
  }
  .questions .notice-icon {
    background: var(--warning);
  }
  .questions.done {
    background: var(--success-soft);
  }
  .questions.done .notice-icon {
    background: var(--success);
  }
  .danger {
    background: var(--danger-soft);
  }
  .danger .notice-icon {
    background: var(--danger);
  }
  .only {
    text-align: center;
    margin-bottom: 12px;
  }
  .toggle {
    margin: 16px auto 0;
    display: flex;
    align-items: center;
    gap: 6px;
    border: none;
    background: none;
    color: var(--accent);
    font-weight: 550;
    cursor: pointer;
  }
  .toggle span {
    display: grid;
    transition: transform 0.2s var(--ease);
  }
  .toggle .open {
    transform: rotate(180deg);
  }
  .files {
    list-style: none;
    margin: 10px 0 0;
    padding: 6px 0;
    border-radius: var(--radius);
    background: var(--surface-2);
    font-size: 14px;
    user-select: text;
  }
  .files li {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    padding: 5px 16px;
  }
  .path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .backdrop {
    position: fixed;
    inset: 0;
    display: grid;
    place-items: center;
    background: rgb(20 15 10 / 40%);
    animation: fade 0.2s var(--ease);
  }
  .dialog {
    width: min(440px, calc(100vw - 48px));
    padding: 28px;
    border-radius: var(--radius-large);
    background: var(--surface);
    box-shadow: var(--shadow);
    text-align: center;
    display: grid;
    gap: 8px;
    justify-items: center;
  }
  .warn-icon {
    color: var(--danger);
  }
  .dialog-actions {
    margin-top: 16px;
    display: flex;
    gap: 12px;
  }
  @keyframes card-in {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
</style>
