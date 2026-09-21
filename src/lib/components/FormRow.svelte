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
    /** `warning` turns the description into the row's error message. */
    tone?: 'default' | 'warning'
    icon?: IconName
    /** Anything else that leads the row, such as an app icon (24px wide). */
    lead?: Snippet
    first?: boolean
    labelFor?: string
    children?: Snippet
  }

  const { title, description, tone = 'default', icon, lead, first = false, labelFor, children }: Props = $props()
</script>

<div class="row" class:first class:described={!!description} class:with-icon={!!icon} class:with-lead={!!lead}>
  {#if icon}
    <span class="icon"><Icon name={icon} /></span>
  {:else if lead}
    {@render lead()}
  {/if}
  <div class="text">
    {#if labelFor}
      <label for={labelFor}>{title}</label>
    {:else}
      <span class="title">{title}</span>
    {/if}
    {#if description}
      <span class="description {tone}" role={tone === 'warning' ? 'alert' : undefined}>{description}</span>
    {/if}
  </div>
  {#if children}
    <div class="control">
      {@render children()}
    </div>
  {/if}
</div>

<style>
  .row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: var(--row-height);
    padding: 8px 16px;
  }

  .described {
    padding-block: 10px;
  }

  /* The hairline starts where the text starts: past the icon when there is one. */
  .row::before {
    content: '';
    position: absolute;
    inset-block-start: 0;
    inset-inline: 16px 0;
    height: 1px;
    background: var(--border-subtle);
  }

  .with-icon::before {
    inset-inline-start: 46px;
  }

  .with-lead::before {
    inset-inline-start: 52px;
  }

  .first::before {
    display: none;
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

  .warning {
    color: var(--state-warning-text);
  }

  .control {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: none;
  }
</style>
