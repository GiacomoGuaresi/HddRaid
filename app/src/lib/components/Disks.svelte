<!--
  Illustrazione dei due dischi, con lo stato di ciascuno e del collegamento
  tra i due. È l'elemento che si vede per primo in quasi ogni schermata.
-->
<script lang="ts">
  import { Check, HeartPulse } from "./icons";

  type DiskState = "off" | "on" | "ok" | "warn";

  interface Props {
    a: DiskState;
    b: DiskState;
    labelA: string;
    labelB: string;
    /** Cosa c'è tra i due dischi. */
    link?: "none" | "sync" | "ok" | "question" | "check";
    /** Più piccola, per le schermate con molto contenuto. */
    compact?: boolean;
  }

  let { a, b, labelA, labelB, link = "none", compact = false }: Props = $props();
</script>

{#snippet disk(state: DiskState, label: string)}
  <figure class="disk {state}">
    <svg viewBox="0 0 120 100" aria-hidden="true">
      <ellipse class="shadow" cx="60" cy="94" rx="46" ry="5" />
      <rect class="body" x="6" y="8" width="108" height="78" rx="16" />
      <rect class="face" x="16" y="18" width="88" height="40" rx="8" />
      <line class="groove" x1="20" y1="72" x2="44" y2="72" />
      <circle class="led" cx="94" cy="72" r="4.5" />
    </svg>
    <figcaption>{label}</figcaption>
  </figure>
{/snippet}

<div class="disks" class:compact>
  {@render disk(a, labelA)}
  <div class="link {link}" aria-hidden="true">
    <svg class="line" viewBox="0 0 100 8" preserveAspectRatio="none">
      <line x1="2" y1="4" x2="98" y2="4" />
    </svg>
    {#if link !== "none" && link !== "sync"}
      <span class="badge">
        {#if link === "ok"}
          <Check size={22} strokeWidth={3} />
        {:else if link === "question"}
          <span class="mark">?</span>
        {:else}
          <HeartPulse size={22} strokeWidth={2.5} />
        {/if}
      </span>
    {/if}
  </div>
  {@render disk(b, labelB)}
</div>

<style>
  .disks {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
  }

  .compact {
    zoom: 0.7;
  }
  .disk {
    margin: 0;
    width: 150px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    transition: opacity 0.3s var(--ease);
  }
  .disk svg {
    width: 144px;
    overflow: visible;
  }
  figcaption {
    font-weight: 600;
    font-size: 16px;
    max-width: 150px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .shadow {
    fill: rgb(60 40 20 / 10%);
  }
  .body {
    fill: var(--surface);
    stroke: var(--border-strong);
    stroke-width: 2;
  }
  .face {
    fill: var(--surface-2);
  }
  .groove {
    stroke: var(--border-strong);
    stroke-width: 3;
    stroke-linecap: round;
  }
  .led {
    fill: var(--text-faint);
  }

  .on .body,
  .ok .body,
  .warn .body {
    filter: drop-shadow(0 4px 10px rgb(60 40 20 / 10%));
  }
  .on .led {
    fill: var(--accent);
    animation: blink 1.4s ease-in-out infinite;
  }
  .ok .led {
    fill: var(--success);
  }
  .warn .led {
    fill: var(--warning);
  }
  .off {
    opacity: 0.55;
  }
  .off .body {
    fill: transparent;
    stroke-dasharray: 6 6;
  }
  .off .face,
  .off .shadow {
    fill: transparent;
  }
  .off .groove {
    stroke: transparent;
  }
  .off figcaption {
    color: var(--text-muted);
  }

  .link {
    position: relative;
    width: 110px;
    height: 48px;
    margin-bottom: 30px;
    display: grid;
    place-items: center;
  }
  .line {
    position: absolute;
    inset: 20px 0;
    width: 100%;
    height: 8px;
  }
  .line line {
    stroke: var(--border-strong);
    stroke-width: 3;
    stroke-linecap: round;
    stroke-dasharray: 2 8;
    vector-effect: non-scaling-stroke;
  }
  .sync .line line {
    stroke: var(--accent);
    stroke-dasharray: 2 10;
    animation: flow 0.9s linear infinite;
  }
  .ok .line line {
    stroke: var(--success);
    stroke-dasharray: none;
  }
  .question .line line {
    stroke: var(--warning);
  }
  .check .line line {
    stroke: var(--accent);
  }

  .badge {
    position: relative;
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    border-radius: 50%;
    color: #fff;
    animation: pop 0.35s var(--ease);
  }
  .ok .badge {
    background: var(--success);
  }
  .mark {
    font-size: 24px;
    font-weight: 800;
    line-height: 1;
  }
  .question .badge {
    background: var(--warning);
  }
  .check .badge {
    background: var(--accent);
    color: var(--on-accent);
    animation: beat 1.6s ease-in-out infinite;
  }

  @keyframes flow {
    to {
      stroke-dashoffset: -12;
    }
  }
  @keyframes blink {
    50% {
      opacity: 0.35;
    }
  }
  @keyframes pop {
    from {
      transform: scale(0.4);
      opacity: 0;
    }
  }
  @keyframes beat {
    50% {
      transform: scale(1.08);
    }
  }
</style>
