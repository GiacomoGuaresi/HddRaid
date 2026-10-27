<!--
  Impostazioni dell'app, nel popup che si apre dal computer al centro. I nomi
  dei dischi si cambiano dal popup di ciascun disco.
-->
<script lang="ts">
  import { api, app, setTheme } from "$lib/app.svelte";
  import Popup from "$lib/components/Popup.svelte";
  import { ChevronRight, History, Settings as Gear } from "$lib/components/icons";
  import { setLanguage, t } from "$lib/i18n/index.svelte";
  import type { Settings } from "$lib/types";

  interface Props {
    onclose: () => void;
  }

  let { onclose }: Props = $props();

  let settings = $state<Settings | null>(null);
  let version = $state("");

  $effect(() => {
    api().settings().then((v) => (settings = v));
    api().version().then((v) => (version = v));
  });

  async function save(patch: Partial<Settings>) {
    if (!settings) return;
    settings = { ...settings, ...patch };
    if ("language" in patch) setLanguage(settings.language);
    if ("theme" in patch) setTheme(settings.theme);
    await api().setSettings($state.snapshot(settings));
  }

  function history() {
    onclose();
    app.view = "history";
  }
</script>

<Popup label={t("settings.title")} {onclose}>
  <header>
    <span class="icon"><Gear size={26} /></span>
    <h2>{t("settings.title")}</h2>
  </header>

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
      <div class="row column">
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
          onchange={(e) =>
            save({ language: (e.currentTarget.value || null) as Settings["language"] })}
        >
          <option value="">{t("settings.languageAuto")}</option>
          <option value="it">Italiano</option>
          <option value="en">English</option>
        </select>
      </div>
      <div class="row">
        <span>{t("settings.theme")}</span>
        <select
          value={settings.theme ?? ""}
          onchange={(e) => save({ theme: (e.currentTarget.value || null) as Settings["theme"] })}
        >
          <option value="">{t("settings.themeAuto")}</option>
          <option value="light">{t("settings.themeLight")}</option>
          <option value="dark">{t("settings.themeDark")}</option>
        </select>
      </div>
    </div>
  {/if}

  {#if app.state?.pair}
    <div class="group">
      <button class="row link" onclick={history}>
        <span class="with-icon"><History size={20} />{t("settings.history")}</span>
        <ChevronRight size={20} />
      </button>
    </div>
  {/if}

  <p class="version muted">{t("settings.version", { version })}</p>
</Popup>

<style>
  header {
    display: flex;
    align-items: center;
    gap: 14px;
    padding-right: 32px;
  }
  .icon {
    display: grid;
    place-items: center;
    width: 52px;
    height: 52px;
    flex: none;
    border-radius: 14px;
    background: var(--accent-soft);
    color: var(--accent);
  }
  .group {
    border-radius: var(--radius);
    border: 1px solid var(--border);
    overflow: hidden;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 14px 16px;
    border: none;
    width: 100%;
    background: none;
    text-align: left;
  }
  .row + .row {
    border-top: 1px solid var(--border);
  }
  .column {
    flex-direction: column;
    align-items: stretch;
    gap: 10px;
  }
  .link {
    cursor: pointer;
    color: var(--text);
  }
  .link:hover {
    background: var(--surface-2);
  }
  .with-icon {
    display: inline-flex;
    align-items: center;
    gap: 10px;
  }
  select {
    font: inherit;
    color: var(--text);
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-small);
    padding: 8px 12px;
  }
  .segmented {
    display: flex;
    padding: 3px;
    border-radius: var(--radius);
    background: var(--surface-2);
  }
  .segmented button {
    flex: 1;
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
    text-align: center;
    font-size: 13px;
  }
</style>
