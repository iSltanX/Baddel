<script lang="ts">
  import Icon from './Icon.svelte'

  /**
   * A pop-up button. Built on `<select>` so it keeps the system's own menu and
   * keyboard behaviour; only the closed control is drawn by us. Its width is the
   * shared control column, and a long value is cut with an ellipsis.
   */
  interface Props {
    value: string
    options: { value: string; label: string }[]
    id?: string
    disabled?: boolean
    /** Shown instead of an empty control when there is nothing to choose from. */
    placeholder?: string
    onchange: (value: string) => void
  }

  const { value, options, id, disabled = false, placeholder, onchange }: Props = $props()
</script>

<span class="popup" class:disabled>
  <select {id} {value} {disabled} onchange={(event) => onchange(event.currentTarget.value)}>
    {#if options.length === 0 && placeholder}
      <option value="">{placeholder}</option>
    {/if}
    {#each options as option (option.value)}
      <option value={option.value}>{option.label}</option>
    {/each}
  </select>
  <span class="chevron"><Icon name="chevronUpDown" size={16} /></span>
</span>

<style>
  .popup {
    position: relative;
    display: inline-flex;
    width: var(--control-column);
    flex: none;
  }

  select {
    width: 100%;
    height: var(--control-height);
    /* Room for the chevron on the trailing side, whichever side that is. */
    padding-inline: 10px 26px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-control);
    background: var(--bg-surface);
    box-shadow: var(--shadow-control);
    font-size: var(--size-body);
    line-height: var(--leading-body);
    text-overflow: ellipsis;
    appearance: none;
    transition: border-color 120ms ease-out;
  }

  select:hover:not(:disabled) {
    border-color: var(--text-tertiary);
  }

  select:focus-visible {
    box-shadow: 0 0 0 var(--focus-width) var(--focus-ring);
  }

  .chevron {
    position: absolute;
    inset-block: 0;
    inset-inline-end: 6px;
    display: grid;
    place-items: center;
    color: var(--text-secondary);
    pointer-events: none;
  }

  .disabled {
    opacity: 0.4;
  }

  .disabled select {
    box-shadow: none;
  }
</style>
