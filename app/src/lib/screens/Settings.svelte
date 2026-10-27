<!-- Impostazioni: poche voci, una sola schermata. -->
<script lang="ts">
  import { api, app, label } from "$lib/app.svelte";
  import Button from "$lib/components/Button.svelte";
  import { HeartPulse, History, HardDrive } from "$lib/components/icons";
  import { relative } from "$lib/format";
  import { setLanguage, t } from "$lib/i18n/index.svelte";
  import type { Settings, Side } from "$lib/types";

  let settings = $state<Settings | null>(null);
  let version = $state("");
  const names = $state<Record<Side, string>>({ A: label("A"), B: label("B") });

  const s = $derived(app.state!);
  const both = $derived(Boolean(s.disks.A && s.disks.B) && !s.busy);

  $effect(() => {
    api().settings().then((v) => (settings = v));
    api().version().then((v) => (version = v));
  });

  async function save(patch: Partial<Settings>) {
    if (!settings) return;
    settings = { ...settings, ...patch };
    if ("language" in patch) setLanguage(settings.language);
    await api().setSettings($state.snapshot(settings));
  }

  function rename(side: Side) {
    const name = names[side].trim();
    if (name && name !== label(side)) api().renameDisk(side, name);
  }

  function checkNow() {
    app.view = "home";
    api().startCheck();
  }
</script>

<section class="settings">
  <h1>{t("settings.title")}</h1>

  {#if settings}
    <div class="group">
      <label class="row">
        <span>{t("settings.autostart")}</span>
        <input
          type="checkbox"
          class="switch"
          checked={settings.autostart}
          onchange={(e) => save({ autostart: e.currentTarget.checked })}
        />
      </label>
      <div class="row">
        <span>{t("settings.checkEvery")}</span>
        <div class="segmented">
          {#each [1, 3, 6] as const as months (months)}
            <button
              class:active={settings.check_months === months}
              onclick={() => save({ check_months: months })}
            >
              {t("settings.months", { count: months })}
            </button>
          {/each}
        </div>
      </div>
      <div class="row">
        <span>{t("settings.language")}</span>
        <select
          value={settings.language ?? ""}
          onchange={(e) => save({ language: (e.currentTarget.value || null) as Settings["language"] })}
        >
          <option value="">{t("settings.languageAuto")}</option>
          <option value="it">Italiano</option>
          <option value="en">English</option>
        </select>
      </div>
    </div>
  {/if}

  {#if s.pair}
    <h2>{t("settings.disks")}</h2>
    <div class="group">
      <div class="row column">
        <span>{t("settings.names")}</span>
        <span class="muted small">{t("settings.namesHint")}</span>
        <div class="names">
          {#each ["A", "B"] as const as side (side)}
            <div class="name">
              <HardDrive size={20} />
              <input
                bind:value={names[side]}
                maxlength="40"
                onblur={() => rename(side)}
                onkeydown={(e) => e.key === "Enter" && e.currentTarget.blur()}
              />
            </div>
          {/each}
        </div>
      </div>
      <div class="row">
        <div class="column">
          <span>{t("settings.checkNow")}</span>
          <span class="muted small">
            {#if !both}
              {t("settings.checkNowHint")}
            {:else if s.last_check}
              {t("settings.lastCheck", { when: relative(s.last_check) })}
            {:else}
              {t("settings.neverChecked")}
            {/if}
          </span>
        </div>
        <Button disabled={!both} onclick={checkNow}>
          <HeartPulse size={18} />
          {t("check.due.start")}
        </Button>
      </div>
      <button class="row link" onclick={() => (app.view = "history")}>
        <span>{t("settings.history")}</span>
        <History size={20} />
      </button>
      <div class="row">
        <div class="column">
          <span>{t("settings.broken")}</span>
          <span class="muted small">{t("settings.brokenSoon")}</span>
        </div>
      </div>
    </div>
  {/if}

  <p class="version muted">{t("settings.version", { version })}</p>
</section>

<style>
  .settings {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 8px 40px 32px;
    max-width: 680px;
    width: 100%;
    margin: 0 auto;
    animation: enter 0.3s var(--ease);
  }
  h1 {
    margin-bottom: 20px;
  }
  h2 {
    margin: 28px 0 12px;
    font-size: 15px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }
  .group {
    border-radius: var(--radius-large);
    background: var(--surface);
    border: 1px solid var(--border);
    overflow: hidden;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 16px 20px;
    border: none;
    width: 100%;
    background: none;
    text-align: left;
  }
  .row + .row {
    border-top: 1px solid var(--border);
  }
  .column {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 2px;
  }
  .small {
    font-size: 14px;
  }
  .link {
    cursor: pointer;
  }
  .link:hover {
    background: var(--surface-2);
  }
  .names {
    display: flex;
    gap: 12px;
    margin-top: 10px;
  }
  .name {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-muted);
  }
  input:not([type]),
  select {
    font: inherit;
    color: var(--text);
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-small);
    padding: 8px 12px;
  }
  .name input {
    flex: 1;
    min-width: 0;
  }
  .segmented {
    display: flex;
    padding: 3px;
    border-radius: var(--radius);
    background: var(--surface-2);
  }
  .segmented button {
    border: none;
    background: none;
    padding: 6px 14px;
    border-radius: var(--radius-small);
    cursor: pointer;
    color: var(--text-muted);
  }
  .segmented .active {
    background: var(--surface);
    color: var(--text);
    font-weight: 600;
    box-shadow: var(--shadow);
  }
  .switch {
    appearance: none;
    width: 46px;
    height: 28px;
    border-radius: 999px;
    background: var(--surface-3);
    position: relative;
    cursor: pointer;
    transition: background 0.2s var(--ease);
    flex: none;
  }
  .switch::after {
    content: "";
    position: absolute;
    top: 3px;
    left: 3px;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 3px rgb(0 0 0 / 25%);
    transition: transform 0.2s var(--ease);
  }
  .switch:checked {
    background: var(--accent);
  }
  .switch:checked::after {
    transform: translateX(18px);
  }
  .version {
    margin-top: 24px;
    text-align: center;
    font-size: 13px;
  }
  @keyframes enter {
    from {
      opacity: 0;
    }
  }
</style>
