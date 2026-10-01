<script lang="ts">
  import type { Snippet } from 'svelte'

  /**
   * `Button` in 02 — Components. One primary per screen. `regular` (MD, 28) lives inside
   * rows and cards, `large` (LG, 36) in window footers, and `plain` is the Ghost variant
   * for quiet actions. Flat: no shadow, the focus ring is the only outline that moves.
   */
  interface Props {
    variant?: 'primary' | 'secondary' | 'plain'
    size?: 'regular' | 'large'
    disabled?: boolean
    loading?: boolean
    type?: 'button' | 'submit'
    onclick?: () => void
    children: Snippet
  }

  const {
    variant = 'secondary',
    size = 'regular',
    disabled = false,
    loading = false,
    type = 'button',
    onclick,
    children,
  }: Props = $props()
</script>

<button class="{variant} {size}" class:loading {type} disabled={disabled || loading} aria-busy={loading} {onclick}>
  {#if loading}<span class="spinner" aria-hidden="true"></span>{/if}
  <span class="label">{@render children()}</span>
</button>

<style>
  button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-8);
    height: var(--control-md);
    min-width: 56px;
    padding: 0 var(--space-12);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-sm);
    background: var(--bg-surface);
    color: var(--text-primary);
    font-size: var(--size-body);
    line-height: var(--leading-body);
    white-space: nowrap;
    transition:
      background-color 120ms ease-out,
      border-color 120ms ease-out;
  }

  .large {
    height: var(--control-lg);
    min-width: 88px;
    padding: 0 var(--space-20);
    border-radius: var(--radius-md);
  }

  /* Hover and pressed are a wash over the surface, so they work on any background. */
  .secondary:hover:not(:disabled) {
    border-color: var(--text-tertiary);
    background: linear-gradient(var(--overlay-hover), var(--overlay-hover)), var(--bg-surface);
  }

  .secondary:active:not(:disabled) {
    border-color: var(--text-tertiary);
    background: linear-gradient(var(--overlay-pressed), var(--overlay-pressed)), var(--bg-surface);
  }

  /* Ink on mint, never white: 4.7:1 where white would be 3.2:1. */
  .primary {
    border-color: transparent;
    background: var(--action-primary);
    color: var(--text-on-primary);
    font-weight: 700;
  }

  .primary:hover:not(:disabled) {
    background: var(--action-primary-hover);
  }

  .primary:active:not(:disabled) {
    background: var(--action-primary-pressed);
  }

  .plain {
    border-color: transparent;
    background: none;
    color: var(--text-brand);
  }

  .plain:hover:not(:disabled) {
    background: var(--overlay-hover);
  }

  .plain:active:not(:disabled) {
    background: var(--overlay-pressed);
  }

  button:focus-visible {
    box-shadow: var(--focus-ring);
  }

  button:disabled:not(.loading) {
    opacity: 0.4;
  }

  /* While it works, the label dims and the spinner leads it. */
  .loading .label {
    opacity: 0.7;
  }

  .spinner {
    width: 12px;
    height: 12px;
    border: 1.5px solid currentColor;
    border-inline-end-color: transparent;
    border-radius: 50%;
    opacity: 0.875;
    animation: spin 700ms linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
