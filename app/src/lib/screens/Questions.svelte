<!-- Le domande, una alla volta: file cancellati e file cambiati su entrambi i dischi. -->
<script lang="ts">
  import { api, app, label, other } from "$lib/app.svelte";
  import Button from "$lib/components/Button.svelte";
  import { File, FileImage, Folder, FolderOpen, Trash, Undo } from "$lib/components/icons";
  import { bytes, date } from "$lib/format";
  import { t } from "$lib/i18n/index.svelte";
  import type { Answer, Question, Side, VersionView } from "$lib/types";

  const questions = $derived(app.state!.summary?.questions ?? []);
  let index = $state(0);
  let sameForAll = $state(false);

  const q = $derived(questions[Math.min(index, questions.length - 1)]);
  const name = $derived(q.path.split("/").pop() ?? q.path);
  const folder = $derived(q.path.split("/").slice(0, -1).join(" › "));
  const isImage = $derived(/\.(jpe?g|png|heic|gif|webp|tiff?|raw|cr2|nef|arw|dng)$/i.test(q.path));

  /** Il nome della versione del disco B con "Tienile entrambe". */
  const altName = $derived.by(() => {
    const dot = name.lastIndexOf(".");
    const suffix = ` (${label("B")})`;
    return dot > 0 ? name.slice(0, dot) + suffix + name.slice(dot) : name + suffix;
  });

  function answer(a: Answer) {
    const targets = sameForAll ? questions.slice(index).filter((x) => x.kind === q.kind) : [q];
    for (const x of targets) app.answers[x.key] = a;
    next();
  }

  function next() {
    const nextIndex = questions.findIndex((x, i) => i > index && !app.answers[x.key]);
    if (nextIndex === -1) {
      app.view = "home";
    } else {
      index = nextIndex;
      sameForAll = false;
    }
  }

  function open(side: Side, reveal = false) {
    api().openFile(side, q.path, reveal);
  }

  function version(question: Question, side: Side): VersionView | null {
    return side === "A" ? question.a : question.b;
  }
</script>

{#snippet thumb(isDir: boolean)}
  <span class="thumb">
    {#if isDir}
      <Folder size={40} />
    {:else if isImage}
      <FileImage size={40} />
    {:else}
      <File size={40} />
    {/if}
  </span>
{/snippet}

<section class="questions">
  <p class="counter muted">{t("questions.counter", { n: index + 1, total: questions.length })}</p>

  {#key q.key}
    <div class="question">
      {#if q.kind === "deletion" && q.deleted_on}
        {@const kept = other(q.deleted_on)}
        {@const v = version(q, kept)}
        <h1>
          {t(q.is_dir ? "questions.deletion.folder" : "questions.deletion.file", {
            disk: label(q.deleted_on),
          })}
        </h1>
        <button class="file card" onclick={() => open(kept)} title={t(q.is_dir ? "questions.openFolder" : "questions.openFile")}>
          {@render thumb(q.is_dir)}
          <div>
            <strong class="name">{name}</strong>
            {#if folder}<span class="muted">{folder}</span>{/if}
            {#if v}
              <span class="muted">
                {bytes(v.size)} · {t("questions.on", { disk: label(kept) })}
              </span>
            {/if}
            {#if q.is_dir}<span class="muted">{t("questions.folderHint")}</span>{/if}
          </div>
        </button>
        {#if !q.is_dir}
          <button class="reveal" onclick={() => open(kept, true)}>
            <FolderOpen size={16} />
            {t("questions.reveal")}
          </button>
        {/if}
        <div class="choices">
          <Button variant="secondary" size="large" onclick={() => answer("delete")}>
            <Trash size={20} />
            {t("questions.deletion.delete", { disk: label(kept) })}
          </Button>
          <Button variant="primary" size="large" onclick={() => answer("restore")}>
            <Undo size={20} />
            {t(q.is_dir ? "questions.deletion.restoreFolder" : "questions.deletion.restoreFile", {
              disk: label(q.deleted_on),
            })}
          </Button>
        </div>
      {:else}
        <h1>{t("questions.conflict.text")}</h1>
        <p class="path muted">{folder ? `${folder} › ` : ""}<strong>{name}</strong></p>
        <div class="versions">
          {#each ["A", "B"] as const as side (side)}
            {@const v = version(q, side)}
            <div class="version card">
              <button class="thumb-button" disabled={!v} onclick={() => open(side)} title={t("questions.openFile")}>
                {@render thumb(false)}
              </button>
              <strong>{t("questions.on", { disk: label(side) })}</strong>
              {#if v}
                <span class="muted">{t("questions.modified", { date: date(v.modified) })}</span>
                <span class="muted">{bytes(v.size)}</span>
                <button class="reveal" onclick={() => open(side, true)}>
                  <FolderOpen size={16} />
                  {t("questions.reveal")}
                </button>
              {:else}
                <span class="muted">{t("questions.missing")}</span>
              {/if}
              <Button
                variant="primary"
                onclick={() => answer(side === "A" ? "keep_a" : "keep_b")}
              >
                {t("questions.conflict.keep")}
              </Button>
            </div>
          {/each}
        </div>
        <div class="choices">
          <Button variant="secondary" onclick={() => answer("both")}>
            {t("questions.conflict.both")}
          </Button>
        </div>
        <p class="hint muted">{t("questions.conflict.bothHint", { disk: label("B"), name: altName })}</p>
      {/if}
    </div>
  {/key}

  <footer>
    {#if questions.length - index > 1}
      <label class="same">
        <input type="checkbox" bind:checked={sameForAll} />
        {t("questions.sameForAll")}
      </label>
    {:else}
      <span></span>
    {/if}
    <Button variant="ghost" onclick={() => answer("later")}>{t("questions.later")}</Button>
  </footer>
</section>

<style>
  .questions {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: 8px 40px 28px;
  }
  .counter {
    text-align: center;
    font-size: 14px;
    font-weight: 550;
  }
  .question {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: safe center;
    gap: 20px;
    text-align: center;
    animation: slide 0.3s var(--ease);
  }
  h1 {
    font-size: 24px;
    max-width: 560px;
  }
  .card {
    border-radius: var(--radius-large);
    background: var(--surface);
    border: 1px solid var(--border);
    box-shadow: var(--shadow);
  }
  .file {
    display: flex;
    align-items: center;
    gap: 18px;
    padding: 18px 24px;
    text-align: left;
    max-width: 520px;
  }
  button.file {
    font: inherit;
    color: inherit;
    cursor: pointer;
    transition:
      transform 0.15s var(--ease),
      box-shadow 0.15s var(--ease);
  }
  button.file:hover,
  .thumb-button:hover:not(:disabled) {
    transform: translateY(-2px);
  }
  .thumb-button {
    border: none;
    background: none;
    padding: 0;
    cursor: pointer;
    transition: transform 0.15s var(--ease);
  }
  .reveal {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    border: none;
    background: none;
    color: var(--accent);
    font-size: 15px;
    font-weight: 550;
    cursor: pointer;
    margin-top: -8px;
  }
  .version .reveal {
    margin-top: 2px;
  }
  .reveal:hover {
    text-decoration: underline;
  }
  .file > div {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .name {
    font-size: 18px;
    word-break: break-word;
  }
  .thumb {
    display: grid;
    place-items: center;
    width: 72px;
    height: 72px;
    flex: none;
    border-radius: var(--radius);
    background: var(--accent-soft);
    color: var(--accent);
  }
  .versions {
    display: flex;
    gap: 16px;
  }
  .version {
    width: 230px;
    padding: 20px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }
  .version :global(.button) {
    margin-top: 12px;
  }
  .choices {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 12px;
  }
  .path {
    margin-top: -10px;
    word-break: break-word;
  }
  .hint {
    margin-top: -10px;
    font-size: 14px;
  }
  footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-top: 16px;
  }
  .same {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }
  .same input {
    width: 18px;
    height: 18px;
    accent-color: var(--accent);
  }
  @keyframes slide {
    from {
      opacity: 0;
      transform: translateX(16px);
    }
  }
</style>
