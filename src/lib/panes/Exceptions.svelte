<script lang="ts">
  import AppListRow from '../components/AppListRow.svelte'
  import GroupCard from '../components/GroupCard.svelte'
  import Icon from '../components/Icon.svelte'
  import { addExcludedApp, excludedApps, removeExcludedApp, t, type AppInfo } from '../state.svelte'

  let apps = $state<AppInfo[]>([])
  let busy = $state(false)

  $effect(() => {
    void excludedApps().then((list) => (apps = list))
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

<GroupCard description={t('settings.exceptions.description')}>
  {#if apps.length === 0}
    <p class="empty">{t('settings.exceptions.emptyState')}</p>
  {:else}
    {#each apps as info, index (info.id)}
      <AppListRow
        {info}
        first={index === 0}
        onremove={async (id) => (apps = await removeExcludedApp(id))}
      />
    {/each}
  {/if}
  <button class="add" type="button" disabled={busy} onclick={add}>
    <span class="plus"><Icon name="plus" size={14} /></span>
    {t('settings.exceptions.addApp')}
  </button>
</GroupCard>

<style>
  .empty {
    margin: 0;
    padding: 14px 16px;
    color: var(--text-secondary);
  }

  .add {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-height: var(--row-height);
    padding: 6px 16px;
    border: none;
    border-top: 1px solid var(--border-subtle);
    background: none;
    color: var(--accent-primary);
    text-align: start;
  }

  .add:hover:not(:disabled) {
    background: var(--overlay-hover);
  }

  .add:disabled {
    opacity: 0.38;
  }

  .plus {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border: 1px dashed var(--border-subtle);
    border-radius: 50%;
  }
</style>
