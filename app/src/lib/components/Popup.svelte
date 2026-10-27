<!-- Finestra sopra la schermata, con lo sfondo scurito e il pulsante per chiuderla. -->
<script lang="ts">
  import type { Snippet } from "svelte";
  import { X } from "./icons";
  import { t } from "$lib/i18n/index.svelte";

  interface Props {
    label: string;
    onclose: () => void;
    wide?: boolean;
    children: Snippet;
  }

  let { label, onclose, wide = false, children }: Props = $props();

  /** Sposta il popup in fondo alla pagina: dentro un elemento animato,
   *  `position: fixed` non sarebbe più relativo alla finestra. */
  function portal(node: HTMLElement) {
    document.body.appendChild(node);
    return { destroy: () => node.remove() };
  }
</script>

<div class="backdrop" role="presentation" onclick={onclose} use:portal>
  <div
    class="popup"
    class:wide
    role="dialog"
    aria-modal="true"
    aria-label={label}
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.key === "Escape" && onclose()}
  >
    <button class="close" onclick={onclose} title={t("common.close")}><X size={20} /></button>
    {@render children()}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 10;
    display: grid;
    place-items: center;
    background: rgb(20 15 10 / 40%);
    animation: fade 0.2s var(--ease);
  }
  .popup {
    position: relative;
    width: min(440px, calc(100vw - 48px));
    max-height: calc(100vh - 48px);
    overflow-y: auto;
    padding: 24px;
    border-radius: var(--radius-large);
    background: var(--surface);
    box-shadow: var(--shadow);
    display: grid;
    gap: 16px;
    text-align: left;
    animation: pop 0.25s var(--ease);
  }
  .wide {
    width: min(560px, calc(100vw - 48px));
  }
  .close {
    position: absolute;
    top: 14px;
    right: 14px;
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border: none;
    border-radius: 50%;
    background: none;
    color: var(--text-muted);
    cursor: pointer;
  }
  .close:hover {
    background: var(--surface-2);
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
  }
</style>
