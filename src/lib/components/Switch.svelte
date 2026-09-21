<script lang="ts">
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
    padding: 2px;
    border: none;
    border-radius: 999px;
    background: var(--track-off);
    transition: background-color 150ms ease-out;
  }

  button:hover:not(:disabled) {
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }

  button[aria-checked='true'] {
    background: var(--accent-primary);
  }

  button[aria-checked='true']:hover:not(:disabled) {
    background: var(--accent-hover);
    box-shadow: none;
  }

  button:focus-visible {
    box-shadow: 0 0 0 var(--focus-width) var(--focus-ring);
  }

  .knob {
    display: block;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--control-knob);
    box-shadow: var(--shadow-knob);
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
