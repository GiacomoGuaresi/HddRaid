<!--
  Illustrazione principale: il portatile al centro, collegato con un cavo ai
  due dischi esterni. Lo schermo del portatile, i cavi e i LED dei dischi
  raccontano lo stato, con un'animazione per ciascuno.
-->
<script lang="ts">
  /** off: scollegato; on: collegato; ok: allineato; warn: c'è un problema;
   *  gone: appena espulso. */
  type DiskState = "off" | "on" | "ok" | "warn" | "gone";

  interface Props {
    a: DiskState;
    b: DiskState;
    labelA: string;
    labelB: string;
    /** Cosa mostra il portatile. */
    link?: "none" | "compare" | "sync" | "ok" | "question" | "check" | "error";
    /** Più piccola, per le schermate con molto contenuto. */
    compact?: boolean;
  }

  let { a, b, labelA, labelB, link = "none", compact = false }: Props = $props();

  // Cavi: dal portatile verso ciascun disco.
  const CABLE_A = "M152 104 C 112 104, 92 120, 92 158";
  const CABLE_B = "M308 104 C 348 104, 368 120, 368 158";
</script>

{#snippet disk(state: DiskState, x: number, label: string, side: "a" | "b")}
  <g class="disk {state} {side}">
    <g class="body-group">
      <ellipse class="shadow" cx={x + 60} cy="226" rx="54" ry="5" />
      <!-- Spessore della lastra. -->
      <rect class="edge" x={x} y="164" width="120" height="56" rx="14" />
      <rect class="body" x={x} y="158" width="120" height="56" rx="14" />
      <rect class="sheen" x={x + 12} y="164" width="96" height="6" rx="3" />
      <rect class="port" x={x + 52} y="155" width="16" height="6" rx="2" />
      <circle class="led" cx={x + 104} cy="200" r="4" />
    </g>
    <text class="label" x={x + 60} y="252">{label}</text>
  </g>
{/snippet}

<svg
  class="scene link-{link}"
  class:compact
  viewBox="0 0 460 262"
  role="img"
  aria-label="{labelA}, {labelB}"
>
  <!-- Cavi -->
  <g class="cable {a}">
    <path class="wire" d={CABLE_A} pathLength="100" />
    <path class="flow to-laptop" d={CABLE_A} pathLength="100" />
  </g>
  <g class="cable {b}">
    <path class="wire" d={CABLE_B} pathLength="100" />
    <path class="flow to-disk" d={CABLE_B} pathLength="100" />
  </g>

  {@render disk(a, 32, labelA, "a")}
  {@render disk(b, 308, labelB, "b")}

  <!-- Portatile -->
  <g class="laptop">
    <rect class="lid" x="160" y="6" width="140" height="94" rx="10" />
    <rect class="display" x="168" y="14" width="124" height="78" rx="5" />
    <path class="base" d="M140 100 H320 L330 110 Q330 114 326 114 H134 Q130 114 130 110 Z" />
    <rect class="notch" x="214" y="100" width="32" height="4" rx="2" />

    <g class="display-content" transform="translate(230 53)">
      {#if link === "none"}
        <g class="idle">
          <rect x="-26" y="-8" width="20" height="16" rx="4" />
          <rect x="6" y="-8" width="20" height="16" rx="4" />
        </g>
      {:else if link === "compare"}
        <g class="compare">
          <rect x="-34" y="-20" width="68" height="6" rx="3" />
          <rect x="-34" y="-6" width="48" height="6" rx="3" />
          <rect x="-34" y="8" width="58" height="6" rx="3" />
          <rect class="beam" x="-44" y="-30" width="88" height="3" rx="1.5" />
        </g>
      {:else if link === "sync"}
        <g class="sync">
          <path d="M-22 -6 H18 M10 -14 L18 -6 L10 2" />
          <path d="M22 10 H-18 M-10 2 L-18 10 L-10 18" />
        </g>
      {:else if link === "ok"}
        <g class="ok">
          <circle class="burst" r="22" />
          <circle class="badge" r="22" />
          <path class="tick" d="M-10 0 L-3 7 L11 -7" pathLength="100" />
        </g>
      {:else if link === "question"}
        <g class="question">
          <circle class="badge" r="22" />
          <text y="9">?</text>
        </g>
      {:else if link === "check"}
        <g class="check">
          <polyline points="-44,0 -18,0 -11,-16 -3,16 5,-8 11,0 44,0" pathLength="100" />
        </g>
      {:else if link === "error"}
        <g class="error">
          <circle class="badge" r="22" />
          <text y="9">!</text>
        </g>
      {/if}
    </g>
  </g>
</svg>

<style>
  .scene {
    width: 420px;
    max-width: 100%;
    overflow: visible;
    display: block;
    margin: 0 auto;
  }
  .compact {
    width: 270px;
  }
  /* Rimpicciolita, i nomi resterebbero minuscoli. */
  .compact .label {
    font-size: 22px;
  }
  .scene * {
    transform-box: fill-box;
  }

  /* --- Portatile ------------------------------------------------------ */
  .lid {
    fill: var(--surface);
    stroke: var(--border-strong);
    stroke-width: 2;
  }
  .display {
    fill: var(--surface-2);
  }
  .base {
    fill: var(--surface-3);
    stroke: var(--border-strong);
    stroke-width: 2;
    stroke-linejoin: round;
  }
  .notch {
    fill: var(--border-strong);
  }
  .laptop {
    animation: rise 0.5s var(--ease) both;
  }

  .idle rect {
    fill: var(--border);
    animation: idle 2.4s ease-in-out infinite;
  }
  .idle rect:last-child {
    animation-delay: 1.2s;
  }

  .compare rect {
    fill: var(--border-strong);
  }
  .compare .beam {
    fill: var(--accent);
    opacity: 0.8;
    animation: scan 1.4s ease-in-out infinite alternate;
  }

  .sync path {
    fill: none;
    stroke: var(--accent);
    stroke-width: 4;
    stroke-linecap: round;
    stroke-linejoin: round;
    animation: nudge 1s ease-in-out infinite;
  }
  .sync path:last-child {
    animation-delay: 0.5s;
  }

  .badge {
    fill: var(--success);
  }
  .ok .badge {
    animation: pop 0.45s var(--ease) both;
  }
  .burst {
    fill: none;
    stroke: var(--success);
    stroke-width: 3;
    animation: burst 0.9s 0.2s var(--ease) both;
  }
  .tick {
    fill: none;
    stroke: #fff;
    stroke-width: 4.5;
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-dasharray: 100;
    animation: draw 0.4s 0.25s var(--ease) both;
  }

  .question .badge {
    fill: var(--warning);
    animation: wobble 2.6s ease-in-out infinite;
  }
  .error .badge {
    fill: var(--danger);
    animation: pop 0.45s var(--ease) both;
  }
  .question text,
  .error text {
    fill: #fff;
    font-size: 26px;
    font-weight: 800;
    text-anchor: middle;
  }

  .check polyline {
    fill: none;
    stroke: var(--accent);
    stroke-width: 3.5;
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-dasharray: 60 140;
    animation: heartbeat 1.8s linear infinite;
  }

  /* --- Cavi ----------------------------------------------------------- */
  .wire,
  .flow {
    fill: none;
    stroke-linecap: round;
  }
  .wire {
    stroke: var(--border-strong);
    stroke-width: 4;
    stroke-dasharray: 100;
    animation: plug 0.6s 0.15s var(--ease) both;
  }
  .cable.off .wire {
    stroke-dasharray: 3 5;
    opacity: 0.45;
    animation: none;
  }
  .cable.gone .wire {
    animation: unplug 0.6s var(--ease) both;
  }
  .cable.ok .wire {
    stroke: color-mix(in srgb, var(--success) 60%, var(--border-strong));
  }
  .flow {
    stroke: var(--accent);
    stroke-width: 4;
    stroke-dasharray: 2 7;
    opacity: 0;
  }
  .link-sync .cable.on .flow,
  .link-compare .cable.on .flow {
    opacity: 1;
  }
  .link-sync .to-laptop {
    animation: flow-in 0.8s linear infinite;
  }
  .link-sync .to-disk {
    animation: flow-out 0.8s linear infinite;
  }
  .link-compare .flow {
    stroke-dasharray: 1.5 12;
    animation: flow-in 1.2s linear infinite;
  }

  /* --- Dischi --------------------------------------------------------- */
  .edge {
    fill: var(--border-strong);
  }
  .body {
    fill: var(--surface);
    stroke: var(--border-strong);
    stroke-width: 2;
  }
  .sheen {
    fill: var(--surface-2);
    opacity: 0.8;
  }
  .port {
    fill: var(--text-faint);
  }
  .led {
    fill: var(--text-faint);
  }
  .shadow {
    fill: rgb(60 40 20 / 12%);
  }
  .label {
    font-size: 16px;
    font-weight: 600;
    fill: var(--text);
    text-anchor: middle;
  }

  .disk .body-group {
    animation: rise 0.5s var(--ease) both;
  }
  .disk.b .body-group {
    animation-delay: 0.08s;
  }
  .on .led {
    fill: var(--accent);
    animation: blink 1.2s ease-in-out infinite;
  }
  .link-sync .on .led {
    animation-duration: 0.35s;
  }
  .ok .led {
    fill: var(--success);
  }
  .warn .led {
    fill: var(--warning);
    animation: blink 0.8s ease-in-out infinite;
  }
  .warn .body {
    stroke: var(--warning);
  }

  .off .body {
    fill: transparent;
    stroke-dasharray: 6 6;
  }
  .off .edge,
  .off .sheen,
  .off .port,
  .off .shadow {
    fill: transparent;
  }
  .off .led {
    fill: var(--border);
  }
  .off .body-group {
    animation:
      rise 0.5s var(--ease) both,
      float 3s 0.5s ease-in-out infinite;
  }
  .disk.b.off .body-group {
    animation-delay: 0.08s, 1.5s;
  }
  .off .label {
    fill: var(--text-muted);
  }
  .gone .body-group {
    animation: away 0.7s 0.2s var(--ease) both;
  }
  .gone .label {
    fill: var(--text-muted);
  }

  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
  }
  @keyframes float {
    50% {
      transform: translateY(-4px);
    }
  }
  @keyframes away {
    to {
      opacity: 0.35;
      transform: translateY(8px) scale(0.94);
    }
  }
  @keyframes plug {
    from {
      stroke-dashoffset: 100;
    }
  }
  @keyframes unplug {
    to {
      stroke-dashoffset: -100;
    }
  }
  @keyframes flow-in {
    to {
      stroke-dashoffset: 9;
    }
  }
  @keyframes flow-out {
    to {
      stroke-dashoffset: -9;
    }
  }
  @keyframes blink {
    50% {
      opacity: 0.3;
    }
  }
  @keyframes idle {
    50% {
      opacity: 0.4;
    }
  }
  @keyframes scan {
    from {
      transform: translateY(0);
    }
    to {
      transform: translateY(54px);
    }
  }
  @keyframes nudge {
    50% {
      transform: translateX(3px);
    }
  }
  @keyframes pop {
    from {
      transform: scale(0.3);
      opacity: 0;
    }
    70% {
      transform: scale(1.08);
    }
  }
  @keyframes burst {
    from {
      transform: scale(1);
      opacity: 0.8;
    }
    to {
      transform: scale(1.9);
      opacity: 0;
    }
  }
  @keyframes draw {
    from {
      stroke-dashoffset: 100;
    }
  }
  @keyframes wobble {
    0%,
    80%,
    100% {
      transform: rotate(0);
    }
    85% {
      transform: rotate(-10deg);
    }
    90% {
      transform: rotate(10deg);
    }
    95% {
      transform: rotate(-5deg);
    }
  }
  @keyframes heartbeat {
    from {
      stroke-dashoffset: 60;
    }
    to {
      stroke-dashoffset: -140;
    }
  }
</style>
