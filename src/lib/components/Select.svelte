<script lang="ts">
  /** A pop-up button. Built on `<select>` so it keeps the system's own behaviour. */
  interface Props {
    value: string
    options: { value: string; label: string }[]
    id?: string
    disabled?: boolean
    onchange: (value: string) => void
  }

  const { value, options, id, disabled = false, onchange }: Props = $props()
</script>

<select {id} {value} {disabled} onchange={(event) => onchange(event.currentTarget.value)}>
  {#each options as option (option.value)}
    <option value={option.value}>{option.label}</option>
  {/each}
</select>

<style>
  select {
    height: 26px;
    min-width: 132px;
    max-width: 240px;
    padding-inline: 8px 24px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-control);
    background: var(--bg-surface);
    font-size: var(--size-body);
    /* The system draws its own chevron; ours would fight it in right-to-left. */
    appearance: auto;
  }

  select:disabled {
    opacity: 0.38;
  }
</style>
