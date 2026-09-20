<script lang="ts">
  import type { Snippet } from 'svelte'
  import Icon from './Icon.svelte'
  import type { IconName } from '../icons'

  /**
   * One row of a grouped form: label on the reading side, control on the other.
   * The hairline above it is inset past the icon, as macOS insets its own.
   */
  interface Props {
    title: string
    description?: string
    icon?: IconName
    first?: boolean
    labelFor?: string
    children: Snippet
  }

  const { title, description, icon, first = false, labelFor, children }: Props = $props()
</script>

<div class="row" class:first>
  {#if icon}
    <span class="icon"><Icon name={icon} /></span>
  {/if}
  <div class="text">
    {#if labelFor}
      <label for={labelFor}>{title}</label>
    {:else}
      <span class="title">{title}</span>
    {/if}
    {#if description}
      <span class="description">{description}</span>
    {/if}
  </div>
  <div class="control">
    {@render children()}
  </div>
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: var(--row-height);
    padding: 8px 16px;
    border-top: 1px solid var(--border-subtle);
  }

  .first {
    border-top: none;
  }

  .icon {
    color: var(--text-secondary);
  }

  .text {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
  }

  .title,
  label {
    font-size: var(--size-body);
    line-height: var(--leading-body);
  }

  .description {
    font-size: var(--size-small);
    line-height: var(--leading-small);
    color: var(--text-secondary);
  }

  .control {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: none;
  }
</style>
