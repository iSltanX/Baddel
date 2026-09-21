<script lang="ts">
  import AppListRow from '../components/AppListRow.svelte'
  import Button from '../components/Button.svelte'
  import Icon from '../components/Icon.svelte'
  import { addExcludedApp, excludedApps, removeExcludedApp, t, type AppInfo } from '../state.svelte'

  let apps = $state<AppInfo[]>([])
  let loaded = $state(false)
  let busy = $state(false)

  $effect(() => {
    void excludedApps().then((list) => {
      apps = list
      loaded = true
    })
  })

  async function add() {
    if (busy) return
    busy = true
    try {
      apps = await addExcludedApp()
    } finally {
      busy = false
    }
  }
</script>

<div class="pane">
  <p class="intro">{t('settings.exceptions.description')}</p>

  {#if loaded && apps.length === 0}
    <div class="card empty">
      <span class="glyph"><Icon name="exceptions" size={24} /></span>
      <div class="text">
        <strong>{t('settings.exceptions.emptyState')}</strong>
        <span>{t('settings.exceptions.emptyStateDescription')}</span>
      </div>
      <Button loading={busy} onclick={add}>{t('settings.exceptions.addApp')}</Button>
    </div>
  {:else}
    <div class="card">
      {#each apps as info, index (info.id)}
        <AppListRow
          {info}
          first={index === 0}
          onremove={async (id) => (apps = await removeExcludedApp(id))}
        />
      {/each}
      <button class="add" type="button" disabled={busy} onclick={add}>
        <span class="plus"><Icon name="plus" size={18} /></span>
        {t('settings.exceptions.addApp')}
      </button>
    </div>
  {/if}
</div>

<style>
  .pane {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .intro {
    margin: 0;
    padding-inline: 16px;
    color: var(--text-secondary);
    font-size: var(--size-small);
    line-height: var(--leading-small);
  }

  .card {
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-card);
    background: var(--bg-surface);
    overflow: hidden;
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 32px 24px;
    text-align: center;
  }

  .glyph {
    display: grid;
    place-items: center;
    width: 48px;
    height: 48px;
    border-radius: 50%;
    background: var(--bg-sunken);
    color: var(--text-secondary);
  }

  .text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .text strong {
    font-size: var(--size-body);
    line-height: var(--leading-body);
  }

  .text span {
    color: var(--text-secondary);
    font-size: var(--size-small);
    line-height: var(--leading-small);
  }

  .add {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    min-height: var(--row-height);
    padding: 6px 16px;
    border: none;
    background: none;
    color: var(--accent-primary);
    font-size: var(--size-body);
    line-height: var(--leading-body);
    text-align: start;
  }

  .add::before {
    content: '';
    position: absolute;
    inset-block-start: 0;
    inset-inline: 52px 0;
    height: 1px;
    background: var(--border-subtle);
  }

  .add:hover:not(:disabled) {
    background: var(--overlay-hover);
  }

  .add:focus-visible {
    box-shadow: inset 0 0 0 var(--focus-width) var(--focus-ring);
  }

  .add:disabled {
    opacity: 0.4;
  }

  .plus {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
  }
</style>
