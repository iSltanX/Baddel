<script lang="ts">
  import Icon from './Icon.svelte'
  import { t, type AppInfo } from '../state.svelte'

  interface Props {
    info: AppInfo
    first?: boolean
    onremove: (id: string) => void
  }

  const { info, first = false, onremove }: Props = $props()
  /** An app that is not installed has no name of its own: its bundle id stands in, and the row says why. */
  const unresolved = $derived(info.name === info.id)
</script>

<div class="row" class:first>
  {#if info.icon}
    <img src={info.icon} alt="" width="24" height="24" />
  {:else}
    <span class="fallback"><Icon name="appGeneric" size={20} /></span>
  {/if}
  <span class="text">
    <bdi class="name">{info.name}</bdi>
    {#if unresolved}
      <span class="id">{t('settings.exceptions.notInstalled')}</span>
    {/if}
  </span>
  <!-- Always in the tab order; visible on hover or focus, as macOS does it. -->
  <button
    class="remove"
    type="button"
    aria-label="{t('common.remove')} — {info.name}"
    onclick={() => onremove(info.id)}
  >
    <Icon name="close" size={12} />
  </button>
</div>

<style>
  .row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: var(--row-height);
    padding: 6px 16px;
    transition: background-color 120ms ease-out;
  }

  .row:hover,
  .row:focus-within {
    background: var(--overlay-hover);
  }

  .row::before {
    content: '';
    position: absolute;
    inset-block-start: 0;
    inset-inline: 52px 0;
    height: 1px;
    background: var(--border-subtle);
  }

  .first::before {
    display: none;
  }

  img,
  .fallback {
    width: 24px;
    height: 24px;
    flex: none;
  }

  .fallback {
    display: grid;
    place-items: center;
    color: var(--text-secondary);
  }

  /* A Latin name is its own left-to-right run, but it still belongs beside the icon:
     the run hugs its text and sits at the row's start, whichever side that is. */
  .text {
    display: flex;
    flex: 1;
    flex-direction: column;
    align-items: flex-start;
    min-width: 0;
  }

  .name,
  .id {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .name {
    font-size: 13px;
    line-height: var(--leading-body);
  }

  .id {
    color: var(--text-secondary);
    font-size: var(--size-caption);
    line-height: var(--leading-small);
  }

  .remove {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    flex: none;
    border: none;
    border-radius: 50%;
    background: var(--overlay-pressed);
    color: var(--text-secondary);
    opacity: 0;
    transition: opacity 120ms ease-out;
  }

  .row:hover .remove,
  .remove:focus-visible {
    opacity: 1;
  }

  .remove:hover {
    color: var(--state-danger);
  }
</style>
