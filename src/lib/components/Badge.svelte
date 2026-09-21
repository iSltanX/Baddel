<script lang="ts">
  /** A status pill. The words carry the meaning; the dot and the colour only support them. */
  interface Props {
    label: string
    tone?: 'success' | 'warning' | 'neutral' | 'accent'
    dot?: boolean
  }

  const { label, tone = 'neutral', dot = true }: Props = $props()
</script>

<span class="badge {tone}">
  {#if dot}<span class="dot"></span>{/if}
  {label}
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 22px;
    flex: none;
    padding: 0 8px;
    border-radius: 999px;
    font-size: var(--size-caption);
    line-height: var(--leading-caption);
    font-weight: 700;
    white-space: nowrap;
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
  }

  .success {
    background: var(--state-success-soft);
    color: var(--state-success);
  }

  .warning {
    background: var(--state-warning-soft);
    color: var(--state-warning-text);
  }

  .neutral {
    background: var(--bg-sunken);
    color: var(--text-secondary);
  }

  .accent {
    background: var(--accent-soft);
    color: var(--accent-primary);
  }
</style>
