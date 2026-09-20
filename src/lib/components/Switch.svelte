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
    width: 38px;
    height: 22px;
    flex: none;
    padding: 2px;
    border: none;
    border-radius: 11px;
    background: var(--track-off);
    transition: background-color 150ms ease-out;
  }

  button[aria-checked='true'] {
    background: var(--accent-primary);
  }

  .knob {
    display: block;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 2px rgb(0 0 0 / 25%);
    /* A logical offset so the knob travels the right way in both directions. */
    margin-inline-start: 0;
    transition: margin-inline-start 150ms ease-out;
  }

  button[aria-checked='true'] .knob {
    margin-inline-start: 16px;
  }

  button:disabled {
    opacity: 0.38;
  }
</style>
