<script lang="ts">
  import Icon from './Icon.svelte'
  import { t, type AppInfo } from '../state.svelte'

  interface Props {
    info: AppInfo
    first?: boolean
    onremove: (id: string) => void
  }

  const { info, first = false, onremove }: Props = $props()
</script>

<div class="row" class:first>
  {#if info.icon}
    <img src={info.icon} alt="" width="24" height="24" />
  {:else}
    <span class="fallback"><Icon name="appGeneric" size={20} /></span>
  {/if}
  <span class="name">{info.name}</span>
  <!-- Always in the tab order; visible on hover or focus, as macOS does it. -->
  <button
    class="remove"
    type="button"
    aria-label="{t('common.remove')} — {info.name}"
    onclick={() => onremove(info.id)}
  >
    <Icon name="close" size={14} />
  </button>
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: var(--row-height);
    padding: 6px 16px;
    border-top: 1px solid var(--border-subtle);
  }

  .first {
    border-top: none;
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

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .remove {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    flex: none;
    border: none;
    border-radius: var(--radius-control);
    background: none;
    color: var(--text-secondary);
    opacity: 0;
  }

  .row:hover .remove,
  .remove:focus-visible {
    opacity: 1;
  }

  .remove:hover {
    background: var(--overlay-hover);
    color: var(--state-danger);
  }
</style>
