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
    /** A third line under the description, such as a disclosure link. */
    detail?: Snippet
    /** A read-only row (a field and its value) sits tighter than a control row. */
    dense?: boolean
    first?: boolean
    labelFor?: string
    children?: Snippet
  }

  const { title, description, tone = 'default', icon, lead, detail, dense = false, first = false, labelFor, children }: Props =
    $props()
</script>

<div
  class="row"
  class:first
  class:dense
  class:described={!!description || !!detail}
  class:with-icon={!!icon}
  class:with-lead={!!lead}
>
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
    {#if detail}{@render detail()}{/if}
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
    gap: var(--space-12);
    min-height: var(--row-min);
    padding: var(--space-8) var(--space-16);
  }

  .described {
    padding-block: var(--space-12);
  }

  .dense {
    min-height: 0;
  }

  /* The hairline starts where the text starts: past the icon when there is one. */
  .row::before {
    content: '';
    position: absolute;
    inset-block-start: 0;
    inset-inline: var(--space-16) 0;
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
    font-size: var(--size-label);
    line-height: var(--leading-label);
    color: var(--text-secondary);
  }

  .warning {
    color: var(--warning-text);
  }

  .control {
    display: flex;
    align-items: center;
    gap: var(--space-8);
    flex: none;
  }
</style>
