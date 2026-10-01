<script lang="ts">
  import Icon from './Icon.svelte'
  import type { IconName } from '../icons'

  /**
   * The settings toolbar: pane buttons with the icon above the label, not a
   * sidebar (HIG · Settings). It is not customizable and always shows which pane
   * is selected. In a right-to-left window the panes start from the right, which
   * the flex direction handles on its own.
   */
  interface Props {
    panes: { id: string; label: string; icon: IconName }[]
    selected: string
    onselect: (id: string) => void
  }

  const { panes, selected, onselect }: Props = $props()
</script>

<div class="toolbar" role="tablist">
  {#each panes as pane (pane.id)}
    <button
      type="button"
      role="tab"
      aria-selected={pane.id === selected}
      class:selected={pane.id === selected}
      onclick={() => onselect(pane.id)}
    >
      <Icon name={pane.icon} size={20} />
      <span>{pane.label}</span>
    </button>
  {/each}
</div>

<style>
  .toolbar {
    display: flex;
    justify-content: center;
    gap: var(--space-4);
    padding: var(--space-4) var(--space-12) var(--space-8);
    border-bottom: 1px solid var(--border-subtle);
    background: var(--bg-chrome);
  }

  /* `PaneTab` in 02 — Components. */
  button {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    width: 80px;
    height: 52px;
    border: none;
    border-radius: var(--radius-md);
    background: none;
    color: var(--text-secondary);
    transition: background-color 120ms ease-out;
  }

  button:hover:not(.selected) {
    background: var(--overlay-hover);
  }

  button:focus-visible {
    box-shadow: var(--focus-ring);
  }

  /* Quiet on purpose: the selected pane is marked, but the content stays the loudest thing.
     The mark is the brand text colour, which holds its contrast on the mint tint. */
  .selected {
    background: var(--bg-selected);
    color: var(--text-brand);
  }

  span {
    font-size: var(--size-caption);
    line-height: var(--leading-caption);
  }

  .selected span {
    font-weight: 700;
  }
</style>
