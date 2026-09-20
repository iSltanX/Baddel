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
      <Icon name={pane.icon} size={22} />
      <span>{pane.label}</span>
    </button>
  {/each}
</div>

<style>
  .toolbar {
    display: flex;
    justify-content: center;
    gap: 2px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border-subtle);
    background: var(--bg-window);
  }

  button {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    width: 64px;
    height: 56px;
    border: none;
    border-radius: var(--radius-control);
    background: none;
    color: var(--text-secondary);
  }

  button:hover:not(.selected) {
    background: var(--overlay-hover);
  }

  .selected {
    background: var(--accent-primary);
    color: var(--accent-on);
  }

  span {
    font-size: var(--size-caption);
    line-height: var(--leading-caption);
  }
</style>
