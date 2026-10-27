<!--
  Illustrazione principale: il portatile al centro, collegato con un cavo ai
  due dischi esterni. Lo schermo del portatile, i cavi e i LED dei dischi
  raccontano lo stato, con un'animazione per ciascuno.
-->
<script lang="ts">
  import { inTauri } from "$lib/api";
  import { t } from "$lib/i18n/index.svelte";
  import SettingsPopup from "./SettingsPopup.svelte";

  /** off: scollegato; on: collegato; ok: allineato; warn: c'è un problema;
   *  gone: appena espulso; new: un disco non ancora usato con HddRaid;
   *  other: un disco di un'altra coppia. */
  type DiskState = "off" | "on" | "ok" | "warn" | "gone" | "new" | "other";

  interface Props {
    a: DiskState;
    b: DiskState;
    labelA: string;
    labelB: string;
    /** Cosa mostra il portatile. */
    link?: "none" | "compare" | "sync" | "ok" | "question" | "check" | "error";
    /** Più piccola, per le schermate con molto contenuto. */
    compact?: boolean;
    /** Clic su un disco collegato: apre le sue informazioni. */
    onselect?: (side: "A" | "B") => void;
  }

  let { a, b, labelA, labelB, link = "none", compact = false, onselect }: Props = $props();

  // Il computer al centro apre le impostazioni dell'app.
  // Demo nel browser: `&settings` lo apre subito.
  let settingsOpen = $state(!inTauri && new URLSearchParams(location.search).has("settings"));

  // Cavi: dal portatile verso ciascun disco.
  const CABLE_A = "M152 110 C 112 110, 92 124, 92 162";
  const CABLE_B = "M308 110 C 348 110, 368 124, 368 162";

  // Gradiente del corpo di portatile e dischi, con un id unico nella pagina.
  const uid = $props.id();
  const shade = `device-${uid}`;
</script>

{#snippet disk(state: DiskState, x: number, label: string, side: "a" | "b")}
  {@const clickable = onselect !== undefined && state !== "off" && state !== "gone"}
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <g
    class="disk {state} {side}"
    class:clickable
    role={clickable ? "button" : undefined}
    tabindex={clickable ? 0 : undefined}
    aria-label={clickable ? label : undefined}
    onclick={() => clickable && onselect?.(side === "a" ? "A" : "B")}
    onkeydown={(e) => clickable && e.key === "Enter" && onselect?.(side === "a" ? "A" : "B")}
  >
    <g class="body-group">
      <ellipse class="shadow" cx={x + 60} cy="231" rx="56" ry="5" />
      <!-- Disegnato su 800×800 e rimpicciolito: faccia superiore in
           prospettiva, frontale con il pannello e i due LED. -->
      <g class="art" transform="translate({x} 133) scale(0.15)">
        <path
          class="body"
          fill="url(#{shade})"
          d="M45 322 L180 212 Q225 178 280 178 H520 Q575 178 620 212 L755 322 Q800 360 800 420 V510 Q800 620 690 620 H110 Q0 620 0 510 V420 Q0 360 45 322 Z"
        />
        <rect class="panel" x="65" y="360" width="670" height="196" rx="45" />
        <circle class="dot" cx="510" cy="462" r="36" />
        <circle class="led" cx="632" cy="462" r="36" />
      </g>
      {#if clickable}
        <!-- Ingranaggio: il disco si apre per vederne le informazioni. -->
        <g class="gear">
          <circle cx={x + 114} cy="166" r="13" />
          <g class="art" transform="translate({x + 105} 157) scale(0.75)">
            <path
              d="M9.671 4.136a2.34 2.34 0 0 1 4.659 0 2.34 2.34 0 0 0 3.319 1.915 2.34 2.34 0 0 1 2.33 4.033 2.34 2.34 0 0 0 0 3.831 2.34 2.34 0 0 1-2.33 4.033 2.34 2.34 0 0 0-3.319 1.915 2.34 2.34 0 0 1-4.659 0 2.34 2.34 0 0 0-3.32-1.915 2.34 2.34 0 0 1-2.33-4.033 2.34 2.34 0 0 0 0-3.831A2.34 2.34 0 0 1 6.35 6.051a2.34 2.34 0 0 0 3.319-1.915"
            />
            <circle cx="12" cy="12" r="3" />
          </g>
        </g>
      {/if}
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
  <defs>
    <linearGradient id={shade} x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" style="stop-color: var(--device-top)" />
      <stop offset="1" style="stop-color: var(--device-bottom)" />
    </linearGradient>
  </defs>

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

  <!-- Portatile: apre le impostazioni -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <g
    class="clickable"
    role="button"
    tabindex="0"
    aria-label={t("common.settings")}
    onclick={() => (settingsOpen = true)}
    onkeydown={(e) => e.key === "Enter" && (settingsOpen = true)}
  >
  <g class="laptop">
    <!-- Schermo -->
    <rect class="lid" fill="url(#{shade})" x="162" y="4" width="136" height="90" rx="10" />
    <rect class="display" x="170" y="12" width="120" height="74" rx="5" />
    <!-- Base vista dall'alto: dietro larga come lo schermo, davanti
         appena più larga, per la prospettiva. -->
    <path
      class="base"
      fill="url(#{shade})"
      d="M166 93 H294 Q298 93 300 96 L312 111 Q314 115 309 115 H151 Q146 115 148 111 L160 96 Q162 93 166 93 Z"
    />
    <path class="keys" d="M171 97 H289 L296 106 H164 Z" />
    <rect class="trackpad" x="216" y="108" width="28" height="4" rx="2" />

    <g class="display-content" transform="translate(230 49)">
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
    <g class="gear">
      <circle cx="296" cy="8" r="13" />
      <g class="art" transform="translate(287 -1) scale(0.75)">
        <path
          d="M9.671 4.136a2.34 2.34 0 0 1 4.659 0 2.34 2.34 0 0 0 3.319 1.915 2.34 2.34 0 0 1 2.33 4.033 2.34 2.34 0 0 0 0 3.831 2.34 2.34 0 0 1-2.33 4.033 2.34 2.34 0 0 0-3.319 1.915 2.34 2.34 0 0 1-4.659 0 2.34 2.34 0 0 0-3.32-1.915 2.34 2.34 0 0 1-2.33-4.033 2.34 2.34 0 0 0 0-3.831A2.34 2.34 0 0 1 6.35 6.051a2.34 2.34 0 0 0 3.319-1.915"
        />
        <circle cx="12" cy="12" r="3" />
      </g>
    </g>
  </g>
</svg>

{#if settingsOpen}
  <SettingsPopup onclose={() => (settingsOpen = false)} />
{/if}

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
  /* Il disegno del disco si scala rispetto al foglio, non a se stesso. */
  .scene .art {
    transform-box: view-box;
  }

  /* --- Portatile ------------------------------------------------------ */
  .lid,
  .base,
  .body {
    stroke: var(--device-edge);
    stroke-width: 2;
    vector-effect: non-scaling-stroke;
  }
  .display {
    fill: var(--device-screen);
  }
  .keys,
  .trackpad {
    fill: var(--device-keys);
    opacity: 0.85;
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
  .panel {
    fill: var(--device-face);
  }
  .dot,
  .led {
    fill: var(--device-led-off);
  }
  .shadow {
    fill: var(--device-shadow);
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
  .clickable {
    cursor: pointer;
    outline: none;
    transition: transform 0.2s var(--ease);
  }
  .clickable:hover,
  .clickable:focus-visible {
    transform: translateY(-4px);
  }
  .clickable:focus-visible .label {
    text-decoration: underline;
  }
  .gear > circle {
    fill: var(--surface);
    stroke: var(--border-strong);
    stroke-width: 1.5;
    filter: drop-shadow(0 1px 2px rgb(60 40 20 / 15%));
    transition: fill 0.2s var(--ease);
  }
  .gear .art * {
    fill: none;
    stroke: var(--text-muted);
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .clickable:hover .gear > circle,
  .clickable:focus-visible .gear > circle {
    fill: var(--accent);
    stroke: var(--accent);
  }
  .clickable:hover .gear .art *,
  .clickable:focus-visible .gear .art * {
    stroke: var(--on-accent);
  }
  .new .panel {
    fill: var(--accent-soft);
  }
  .new .led {
    fill: var(--accent);
  }
  .new .label {
    fill: var(--accent);
  }
  .other .panel {
    fill: var(--warning-soft);
  }
  .other .led {
    fill: var(--warning);
  }
  .other .label {
    fill: var(--warning);
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


  .off .body {
    fill: transparent;
    stroke: var(--border-strong);
    stroke-width: 2;
    stroke-dasharray: 6 6;
    vector-effect: non-scaling-stroke;
  }
  .off .panel,
  .off .dot,
  .off .shadow {
    fill: transparent;
  }
  .off .led {
    fill: transparent;
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
