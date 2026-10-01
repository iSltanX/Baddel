<script lang="ts">
  /** `Toggle` in 02 — Components: 36×20, a 16px knob with a hairline, no shadow. */
  interface Props {
    checked: boolean
    disabled?: boolean
    label: string
    onchange: (value: boolean) => void
  }

  const { checked, disabled = false, label, onchange }: Props = $props()
</script>

<button
  type="button"
  role="switch"
  aria-checked={checked}
  aria-label={label}
  {disabled}
  onclick={() => onchange(!checked)}
>
  <span class="knob"></span>
</button>

<style>
  button {
    width: 36px;
    height: 20px;
    flex: none;
    padding: var(--space-2);
    border: none;
    border-radius: var(--radius-full);
    background: var(--control-track-off);
    transition: background-color 150ms ease-out;
  }

  button:hover:not(:disabled) {
    box-shadow: inset 0 0 0 1px var(--border-default);
  }

  button[aria-checked='true'] {
    background: var(--action-primary);
  }

  button[aria-checked='true']:hover:not(:disabled) {
    background: var(--action-primary-hover);
    box-shadow: none;
  }

  button:focus-visible,
  button:focus-visible:hover {
    box-shadow: var(--focus-ring);
  }

  .knob {
    display: block;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--control-knob);
    box-shadow: inset 0 0 0 0.5px var(--border-default);
    /* A logical offset so the knob travels the right way in both directions. */
    margin-inline-start: 0;
    transition: margin-inline-start 150ms ease-out;
  }

  button[aria-checked='true'] .knob {
    margin-inline-start: 16px;
  }

  button:disabled {
    opacity: 0.4;
  }
</style>
