<!--
  Una schermata: illustrazione, titolo, testo, contenuto e pulsanti. Una
  domanda per schermata, con uno o due pulsanti grandi.
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    title: string;
    text?: string;
    tone?: "neutral" | "success" | "warning" | "danger";
    hero?: Snippet;
    children?: Snippet;
    actions?: Snippet;
  }

  let { title, text, tone = "neutral", hero, children, actions }: Props = $props();
</script>

<section class="screen {tone}">
  <div class="content">
    {#if hero}
      <div class="hero">{@render hero()}</div>
    {/if}
    <h1>{title}</h1>
    {#if text}
      <p class="text">{text}</p>
    {/if}
    {#if children}
      <div class="body">{@render children()}</div>
    {/if}
  </div>
  {#if actions}
    <div class="actions">{@render actions()}</div>
  {/if}
</section>

<style>
  .screen {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: 8px 40px 32px;
    animation: enter 0.35s var(--ease);
  }
  .content {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: safe center;
    text-align: center;
    padding: 8px 0 24px;
  }
  .hero {
    margin: 8px 0 24px;
  }
  .text {
    margin-top: 10px;
    max-width: 520px;
    font-size: 17px;
    color: var(--text-muted);
  }
  .success h1 {
    color: var(--success);
  }
  .body {
    width: 100%;
    max-width: 600px;
    margin-top: 24px;
    text-align: left;
  }
  .actions {
    display: flex;
    justify-content: center;
    flex-wrap: wrap;
    gap: 12px;
    padding-top: 20px;
  }

  @keyframes enter {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }
</style>
