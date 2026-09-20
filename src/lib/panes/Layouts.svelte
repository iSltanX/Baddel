<script lang="ts">
  import Button from '../components/Button.svelte'
  import FormRow from '../components/FormRow.svelte'
  import GroupCard from '../components/GroupCard.svelte'
  import Icon from '../components/Icon.svelte'
  import Select from '../components/Select.svelte'
  import {
    app,
    keyboardMap,
    listLayouts,
    openKeyboardSettings,
    save,
    t,
    type KeyboardMap,
    type LayoutEntry,
    type Settings,
  } from '../state.svelte'

  const settings = $derived(app.settings as Settings)

  let layouts = $state<LayoutEntry[]>([])
  let map = $state<KeyboardMap | null>(null)

  async function refresh() {
    ;[layouts, map] = await Promise.all([listLayouts(), keyboardMap()])
  }

  $effect(() => {
    // Re-read whenever the chosen layouts change.
    void settings.arabicLayout
    void settings.latinLayout
    void refresh()
  })

  const arabic = $derived(layouts.filter((entry) => entry.arabic))
  const latin = $derived(layouts.filter((entry) => !entry.arabic))
  /** No explicit choice means "whatever macOS has enabled first". */
  const synced = $derived(settings.arabicLayout === '' && settings.latinLayout === '')

  const options = (entries: LayoutEntry[]) => entries.map((entry) => ({ value: entry.id, label: entry.name }))
</script>

{#if synced && map?.arabicName}
  <p class="badge"><span class="dot"></span>{t('settings.layouts.syncedBadge')}</p>
{/if}

<GroupCard>
  <FormRow first icon="layouts" title={t('settings.layouts.arabicLayout')} labelFor="arabic-layout">
    <Select
      id="arabic-layout"
      value={settings.arabicLayout || (map?.arabicName ? arabic[0]?.id : '') || ''}
      options={options(arabic)}
      disabled={arabic.length === 0}
      onchange={(arabicLayout) => save({ arabicLayout })}
    />
  </FormRow>
  <FormRow icon="keyboard" title={t('settings.layouts.latinLayout')} labelFor="latin-layout">
    <Select
      id="latin-layout"
      value={settings.latinLayout || latin[0]?.id || ''}
      options={options(latin)}
      disabled={latin.length === 0}
      onchange={(latinLayout) => save({ latinLayout })}
    />
  </FormRow>
</GroupCard>

{#if map && map.rows.length > 0}
  <GroupCard title={t('settings.layouts.keyboardMap.title')}>
    <div class="map">
      {#each map.rows as row, index (index)}
        <div class="row">
          {#each row as key (key.latin + key.arabic)}
            <!-- The keyboard is a real object: its rows are never mirrored. -->
            <span class="key" class:multi={key.multi && map.hasMultiChar}>
              <span class="latin">{key.latin}</span>
              <span class="arabic">{key.arabic}</span>
            </span>
          {/each}
        </div>
      {/each}
    </div>
    {#if map.hasMultiChar}
      <p class="note">{t('settings.layouts.keyboardMap.multiCharKeyCaption')}</p>
    {/if}
  </GroupCard>
{:else}
  <div class="empty" role="status">
    <span class="warn"><Icon name="warning" size={18} /></span>
    <p>{t('settings.layouts.emptyState.message')}</p>
    <Button variant="primary" onclick={openKeyboardSettings}>
      {t('settings.layouts.emptyState.action')}
    </Button>
  </div>
{/if}

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    align-self: flex-start;
    margin: 0;
    padding: 3px 10px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--state-success) 14%, transparent);
    color: var(--state-success);
    font-size: var(--size-small);
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
  }

  .map {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 14px;
    direction: ltr;
    background: var(--bg-sunken);
  }

  .row {
    display: flex;
    justify-content: center;
    gap: 5px;
  }

  .key {
    position: relative;
    width: 36px;
    height: 34px;
    flex: none;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-control);
    background: var(--bg-surface);
  }

  .multi {
    border-color: var(--accent-primary);
    background: var(--accent-soft);
  }

  .latin {
    position: absolute;
    top: 3px;
    left: 5px;
    font-family: var(--font-mono);
    font-size: 9px;
    color: var(--text-secondary);
  }

  .arabic {
    position: absolute;
    right: 5px;
    bottom: 2px;
    font-size: 12px;
    color: var(--text-primary);
  }

  .note {
    margin: 0;
    padding: 10px 14px;
    border-top: 1px solid var(--border-subtle);
    color: var(--text-secondary);
    font-size: var(--size-small);
    line-height: var(--leading-small);
  }

  .empty {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 16px;
    border: 1px solid color-mix(in srgb, var(--state-warning) 40%, var(--border-subtle));
    border-radius: var(--radius-card);
    background: color-mix(in srgb, var(--state-warning) 8%, var(--bg-surface));
  }

  .warn {
    color: var(--state-warning);
    display: grid;
    place-items: center;
  }

  .empty p {
    margin: 0;
    flex: 1;
  }
</style>
