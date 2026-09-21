<script lang="ts">
  import type { Snippet } from 'svelte'

  /**
   * One primary button per screen. `regular` (28) lives inside rows and cards,
   * `large` (36) in the onboarding footer, and `plain` is for quiet actions.
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
  {@render children()}
</button>

<style>
  button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: var(--control-height);
    min-width: 56px;
    padding: 0 12px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-control);
    background: var(--bg-surface);
    box-shadow: var(--shadow-control);
    color: var(--text-primary);
    font-size: var(--size-body);
    line-height: var(--leading-body);
    white-space: nowrap;
    transition:
      background-color 120ms ease-out,
      border-color 120ms ease-out;
  }

  .large {
    height: var(--control-height-large);
    min-width: 88px;
    padding: 0 20px;
    border-radius: var(--radius-tab);
  }

  /* Hover and pressed are an overlay on the surface, so they work on any background. */
  .secondary:hover:not(:disabled) {
    border-color: var(--text-tertiary);
    background: linear-gradient(var(--overlay-hover), var(--overlay-hover)), var(--bg-surface);
  }

  .secondary:active:not(:disabled) {
    border-color: var(--text-tertiary);
    background: linear-gradient(var(--overlay-pressed), var(--overlay-pressed)), var(--bg-surface);
  }

  .primary {
    border-color: transparent;
    background: var(--accent-primary);
    box-shadow: none;
    color: var(--accent-on);
    font-weight: 700;
  }

  .primary:hover:not(:disabled) {
    background: var(--accent-hover);
  }

  .primary:active:not(:disabled) {
    background: var(--accent-pressed);
  }

  .plain {
    border-color: transparent;
    background: none;
    box-shadow: none;
    color: var(--accent-primary);
  }

  .plain:hover:not(:disabled) {
    background: var(--overlay-hover);
  }

  .plain:active:not(:disabled) {
    background: var(--overlay-pressed);
  }

  button:focus-visible {
    box-shadow: 0 0 0 var(--focus-width) var(--focus-ring);
  }

  button:disabled:not(.loading) {
    opacity: 0.4;
    box-shadow: none;
  }

  .spinner {
    width: 12px;
    height: 12px;
    border: 1.5px solid currentColor;
    border-inline-end-color: transparent;
    border-radius: 50%;
    opacity: 0.7;
    animation: spin 700ms linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
