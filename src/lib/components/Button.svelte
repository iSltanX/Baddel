<script lang="ts">
  import type { Snippet } from 'svelte'

  interface Props {
    variant?: 'primary' | 'secondary' | 'plain'
    size?: 'regular' | 'large'
    disabled?: boolean
    type?: 'button' | 'submit'
    onclick?: () => void
    children: Snippet
  }

  const {
    variant = 'secondary',
    size = 'regular',
    disabled = false,
    type = 'button',
    onclick,
    children,
  }: Props = $props()
</script>

<button class="{variant} {size}" {type} {disabled} {onclick}>
  {@render children()}
</button>

<style>
  button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 28px;
    padding: 0 12px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-control);
    background: var(--bg-surface);
    color: var(--text-primary);
    font-size: var(--size-body);
    line-height: 1;
    white-space: nowrap;
    transition: background-color 120ms ease-out;
  }

  .large {
    height: 36px;
    padding: 0 18px;
  }

  button:hover:not(:disabled) {
    background: var(--overlay-hover);
  }

  button:active:not(:disabled) {
    background: var(--overlay-pressed);
  }

  .primary {
    border-color: transparent;
    background: var(--accent-primary);
    color: var(--accent-on);
  }

  .primary:hover:not(:disabled),
  .primary:active:not(:disabled) {
    background: var(--accent-primary);
    filter: brightness(1.08);
  }

  .plain {
    border-color: transparent;
    background: none;
    color: var(--accent-primary);
  }

  button:disabled {
    opacity: 0.38;
  }
</style>
