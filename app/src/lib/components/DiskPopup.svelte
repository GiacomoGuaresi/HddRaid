<!--
  Informazioni su un disco: nome (modificabile), file system, spazio, e la
  formattazione per rimetterlo nella coppia o per usare un disco nuovo al
  posto di uno rotto. Formattare chiede di scrivere il nome del disco.
-->
<script lang="ts">
  import { api, app, label, other } from "$lib/app.svelte";
  import Button from "$lib/components/Button.svelte";
  import Popup from "$lib/components/Popup.svelte";
  import { HardDrive, TriangleAlert } from "$lib/components/icons";
  import { bytes } from "$lib/format";
  import { t } from "$lib/i18n/index.svelte";
  import type { Side, VolumeView } from "$lib/types";

  type Target = { kind: "pair"; side: Side } | { kind: "volume"; volume: VolumeView };

  interface Props {
    target: Target;
    onclose: () => void;
  }

  let { target, onclose }: Props = $props();

  const s = $derived(app.state!);
  const pairDisk = $derived(target.kind === "pair" ? s.disks[target.side] : null);
  const info = $derived(
    target.kind === "pair"
      ? {
          name: pairDisk?.label ?? label(target.side),
          root: pairDisk?.root ?? "",
          filesystem: pairDisk?.filesystem ?? null,
          total: pairDisk?.total ?? 0,
          free: pairDisk?.free ?? 0,
        }
      : target.volume,
  );
  const used = $derived(Math.max(0, info.total - info.free));
  const usedPercent = $derived(info.total > 0 ? (used / info.total) * 100 : 0);

  /** Il lato che il disco occuperà dopo la formattazione, se si può. */
  const formatSide = $derived.by((): Side | null => {
    if (target.kind === "pair") {
      return s.disks[other(target.side)] ? target.side : null;
    }
    const present = (["A", "B"] as Side[]).filter((x) => s.disks[x]);
    return present.length === 1 ? other(present[0]) : null;
  });
  /** Un disco della coppia si formatta solo con l'altro collegato, per
   *  poterci ricopiare i file; gli altri dischi si formattano sempre. */
  const formatBlocked = $derived(
    target.kind === "pair" && !formatSide
      ? t("disk.needOther", { disk: label(other(target.side)) })
      : null,
  );

  const fsText = $derived.by(() => {
    switch (info.filesystem) {
      case "exfat":
        return t("disk.fs.exfat");
      case "ntfs":
        return t("disk.fs.ntfs");
      case "apfs":
      case "hfs":
        return t("disk.fs.mac");
      case null:
        return t("disk.fs.unknown");
      default:
        return info.filesystem.toUpperCase();
    }
  });

  const status = $derived(
    target.kind === "pair"
      ? t("disk.inPair")
      : target.volume.other_pair
        ? t("disk.otherPair")
        : t("disk.notUsed"),
  );

  // --- Nome ---------------------------------------------------------------
  let name = $state("");
  $effect(() => {
    name = info.name;
  });
  function labelError(value: string): string | null {
    if (!value.trim()) return t("disk.name.empty");
    if (value.length > 11) return t("disk.name.long");
    if (/["*/:<>?\\|]/.test(value)) return t("disk.name.chars");
    return null;
  }
  const nameError = $derived(labelError(name));

  function rename() {
    if (target.kind !== "pair" || nameError || name === info.name) return;
    api().renameDisk(target.side, name.trim());
  }

  // --- Formattazione ------------------------------------------------------
  let confirming = $state(false);
  let newName = $state("");
  let typed = $state("");
  const newNameError = $derived(labelError(newName));

  function startFormat() {
    newName = (formatSide ? label(formatSide) : info.name).slice(0, 11);
    typed = "";
    confirming = true;
  }

  function format() {
    if (formatBlocked || newNameError || typed !== info.name) return;
    api().formatDisk(info.root, formatSide, newName.trim());
    onclose();
  }
</script>

<Popup label={info.name} {onclose}>

    {#if !confirming}
      <header>
        <span class="icon"><HardDrive size={26} /></span>
        <div>
          <h2>{info.name}</h2>
          <span class="muted">{status}</span>
        </div>
      </header>

      <div class="space">
        <div class="bar"><div style:width="{usedPercent}%"></div></div>
        <div class="space-text muted">
          <span>{t("disk.used", { size: bytes(used) })}</span>
          <span>{t("disk.free", { size: bytes(info.free), total: bytes(info.total) })}</span>
        </div>
      </div>

      <p class="row">
        <span class="muted">{t("disk.filesystem")}</span>
        <span class:warn={info.filesystem !== null && info.filesystem !== "exfat"}>{fsText}</span>
      </p>

      {#if target.kind === "pair"}
        <div class="rename">
          <label for="disk-name" class="muted">{t("disk.name.label")}</label>
          <div class="rename-row">
            <input
              id="disk-name"
              bind:value={name}
              maxlength="11"
              onkeydown={(e) => e.key === "Enter" && rename()}
            />
            <Button disabled={nameError !== null || name === info.name} onclick={rename}>
              {t("settings.rename")}
            </Button>
          </div>
          <span class="hint" class:error={nameError}>{nameError ?? t("disk.name.hint")}</span>
        </div>
      {/if}

      <div class="format">
        {#if formatBlocked}
          <p class="muted hint">{formatBlocked}</p>
        {:else}
          <Button variant="secondary" onclick={startFormat}>
            {target.kind === "volume" && formatSide
              ? t("disk.formatAs", { disk: label(formatSide) })
              : t("disk.format")}
          </Button>
        {/if}
      </div>
    {:else}
      <span class="danger-icon"><TriangleAlert size={30} /></span>
      <h2 class="center">{t("disk.confirm.title", { disk: info.name })}</h2>
      <p class="muted center">
        {formatSide
          ? t("disk.confirm.text", { other: label(other(formatSide)) })
          : t("disk.confirm.plain")}
      </p>

      <label class="field">
        <span class="muted">{t("disk.confirm.newName")}</span>
        <input bind:value={newName} maxlength="11" />
        <span class="hint" class:error={newNameError}>{newNameError ?? t("disk.name.hint")}</span>
      </label>
      <label class="field">
        <span class="muted">{t("disk.confirm.type", { disk: info.name })}</span>
        <input bind:value={typed} placeholder={info.name} />
      </label>

      <div class="actions">
        <Button onclick={() => (confirming = false)}>{t("questions.confirm.cancel")}</Button>
        <Button
          variant="danger"
          disabled={typed !== info.name || newNameError !== null}
          onclick={format}
        >
          {t("disk.confirm.ok")}
        </Button>
      </div>
    {/if}
</Popup>

<style>
  header {
    display: flex;
    align-items: center;
    gap: 14px;
    padding-right: 32px;
  }
  header div {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  h2 {
    word-break: break-word;
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
  .bar {
    height: 10px;
    border-radius: 999px;
    background: var(--surface-3);
    overflow: hidden;
  }
  .bar div {
    height: 100%;
    border-radius: inherit;
    background: var(--accent);
  }
  .space-text {
    display: flex;
    justify-content: space-between;
    margin-top: 6px;
    font-size: 14px;
  }
  .row {
    display: flex;
    justify-content: space-between;
    gap: 16px;
  }
  .warn {
    color: var(--warning);
    font-weight: 600;
  }
  .rename,
  .field {
    display: grid;
    gap: 6px;
  }
  .rename-row {
    display: flex;
    gap: 8px;
  }
  input {
    flex: 1;
    min-width: 0;
    font: inherit;
    color: var(--text);
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-small);
    padding: 8px 12px;
  }
  .hint {
    font-size: 13px;
    color: var(--text-muted);
  }
  .hint.error {
    color: var(--danger);
  }
  .format {
    padding-top: 12px;
    border-top: 1px solid var(--border);
    display: flex;
    justify-content: center;
  }
  .format .hint {
    text-align: center;
  }
  .danger-icon {
    justify-self: center;
    color: var(--danger);
  }
  .center {
    text-align: center;
  }
  .actions {
    display: flex;
    justify-content: center;
    gap: 12px;
  }
</style>
