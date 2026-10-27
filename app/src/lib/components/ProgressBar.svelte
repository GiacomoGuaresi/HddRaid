<script lang="ts">
  interface Props {
    /** Da 0 a 100. */
    value: number;
  }

  let { value }: Props = $props();
</script>

<div
  class="track"
  role="progressbar"
  aria-valuemin={0}
  aria-valuemax={100}
  aria-valuenow={Math.round(value)}
>
  <div class="fill" style:width="{Math.max(2, value)}%"></div>
</div>

<style>
  .track {
    height: 14px;
    border-radius: 999px;
    background: var(--surface-3);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    border-radius: inherit;
    background: linear-gradient(
      90deg,
      var(--accent) 0%,
      color-mix(in srgb, var(--accent) 70%, white) 50%,
      var(--accent) 100%
    );
    background-size: 200% 100%;
    animation: shimmer 2s linear infinite;
    transition: width 0.4s var(--ease);
  }

  @keyframes shimmer {
    to {
      background-position: -200% 0;
    }
  }
</style>
