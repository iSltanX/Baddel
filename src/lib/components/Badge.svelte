<script lang="ts">
  /**
   * `Badge` in 02 — Components. The words carry the meaning; the dot and the colour only
   * support them. `attention` is the coral dot: the one thing on screen that needs a decision.
   */
  interface Props {
    label: string
    tone?: 'success' | 'warning' | 'neutral' | 'brand' | 'attention' | 'danger'
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
    gap: var(--space-8);
    height: 22px;
    flex: none;
    padding: 0 var(--space-8);
    border-radius: var(--radius-full);
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
    background: var(--success-soft);
    color: var(--success);
  }

  .warning {
    background: var(--warning-soft);
    color: var(--warning-text);
  }

  .neutral {
    background: var(--bg-surface-secondary);
    color: var(--text-secondary);
  }

  .brand {
    background: var(--bg-selected);
    color: var(--text-brand);
  }

  .attention {
    background: var(--bg-surface-secondary);
    color: var(--text-primary);
  }

  .attention .dot {
    background: var(--accent);
  }

  .danger {
    background: var(--danger-soft);
    color: var(--danger);
  }
</style>
