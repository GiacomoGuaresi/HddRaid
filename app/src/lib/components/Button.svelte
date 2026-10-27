<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";

  interface Props extends HTMLButtonAttributes {
    variant?: "primary" | "secondary" | "ghost" | "danger";
    size?: "normal" | "large";
    children: Snippet;
  }

  let { variant = "secondary", size = "normal", children, ...rest }: Props = $props();
</script>

<button class="button {variant} {size}" {...rest}>
  {@render children()}
</button>

<style>
  .button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    border: 1px solid transparent;
    border-radius: var(--radius);
    padding: 9px 18px;
    font-weight: 550;
    line-height: 1.25;
    cursor: pointer;
    transition:
      background 0.15s var(--ease),
      border-color 0.15s var(--ease),
      transform 0.1s var(--ease);
  }
  .button:active:not(:disabled) {
    transform: scale(0.98);
  }
  .button:disabled {
    cursor: default;
    opacity: 0.5;
  }
  .large {
    padding: 14px 32px;
    font-size: 18px;
    font-weight: 650;
    border-radius: var(--radius-large);
    min-width: 200px;
  }

  .primary {
    background: var(--accent);
    color: var(--on-accent);
    box-shadow: 0 4px 14px color-mix(in srgb, var(--accent) 30%, transparent);
  }
  .primary:hover:not(:disabled) {
    background: var(--accent-hover);
  }
  .primary:active:not(:disabled) {
    background: var(--accent-pressed);
  }

  .secondary {
    background: var(--surface);
    border-color: var(--border-strong);
  }
  .secondary:hover:not(:disabled) {
    background: var(--surface-2);
  }

  .ghost {
    background: transparent;
    color: var(--text-muted);
  }
  .ghost:hover:not(:disabled) {
    background: var(--surface-2);
    color: var(--text);
  }

  .danger {
    background: var(--danger);
    color: #fff;
  }
  .danger:hover:not(:disabled) {
    background: color-mix(in srgb, var(--danger) 88%, black);
  }
</style>
