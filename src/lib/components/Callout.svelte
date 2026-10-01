<script lang="ts">
  import type { Snippet } from 'svelte'
  import Icon from './Icon.svelte'
  import type { IconName } from '../icons'

  /**
   * `Callout` in 02 — Components: a tinted block that says what happened and, when there
   * is one, what to do about it. The icon leads, the action trails.
   */
  interface Props {
    tone?: 'info' | 'success' | 'warning' | 'danger'
    title: string
    body?: string
    /** A secondary button on the far side. */
    action?: Snippet
  }

  const { tone = 'info', title, body, action }: Props = $props()

  const ICONS: Record<NonNullable<Props['tone']>, IconName> = {
    info: 'info',
    success: 'checkCircle',
    warning: 'warning',
    danger: 'warning',
  }
</script>

<div class="callout {tone}" role={tone === 'warning' || tone === 'danger' ? 'alert' : 'status'}>
  <span class="icon"><Icon name={ICONS[tone]} size={16} /></span>
  <div class="text">
    <strong>{title}</strong>
    {#if body}<span>{body}</span>{/if}
  </div>
  {#if action}{@render action()}{/if}
</div>

<style>
  .callout {
    display: flex;
    align-items: center;
    gap: var(--space-12);
    padding: var(--space-12) var(--space-16);
    border-radius: var(--radius-md);
    text-align: start;
  }

  .icon {
    display: grid;
    flex: none;
  }

  .text {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: var(--space-2);
    min-width: 0;
  }

  strong {
    font-size: var(--size-body);
    line-height: var(--leading-body);
    font-weight: 700;
  }

  span {
    font-size: var(--size-label);
    line-height: var(--leading-label);
  }

  .info {
    background: var(--bg-selected);
  }

  .info .icon {
    color: var(--text-brand);
  }

  .success {
    background: var(--success-soft);
  }

  .success .icon {
    color: var(--success);
  }

  .info span,
  .success span {
    color: var(--text-secondary);
  }

  .warning {
    background: var(--warning-soft);
    color: var(--warning-text);
  }

  .warning .icon {
    color: var(--warning);
  }

  .danger {
    background: var(--danger-soft);
    color: var(--danger);
  }
</style>
