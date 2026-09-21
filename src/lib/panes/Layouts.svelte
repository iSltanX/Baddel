<script lang="ts">
  import Badge from '../components/Badge.svelte'
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

  /** No Arabic layout enabled in macOS: nothing to convert to, so the pane says how to fix it. */
  const missingArabic = $derived(map !== null && map.rows.length === 0)
  const pairName = $derived(map?.arabicName && map?.latinName ? `${map.arabicName}  ⇄  ${map.latinName}` : '')

  const options = (entries: LayoutEntry[]) => entries.map((entry) => ({ value: entry.id, label: entry.name }))
</script>

{#if missingArabic}
  <div class="alert" role="status">
    <span class="warn"><Icon name="warning" size={18} /></span>
    <div class="text">
      <strong>{t('settings.layouts.emptyState.message')}</strong>
      <span>{t('settings.layouts.emptyState.description')}</span>
    </div>
    <Button variant="primary" onclick={openKeyboardSettings}>
      {t('settings.layouts.emptyState.action')}
    </Button>
  </div>
{/if}

<GroupCard title={t('settings.layouts.groupTitle')}>
  {#snippet trailing()}
    {#if missingArabic}
      <Badge tone="warning" label={t('settings.layouts.needsSetupBadge')} />
    {:else if synced && map?.arabicName}
      <Badge tone="success" label={t('settings.layouts.syncedBadge')} />
    {/if}
  {/snippet}
  <FormRow first icon="layouts" title={t('settings.layouts.arabicLayout')} labelFor="arabic-layout">
    <Select
      id="arabic-layout"
      value={settings.arabicLayout || (map?.arabicName ? arabic[0]?.id : '') || ''}
      options={options(arabic)}
      placeholder={t('settings.layouts.unavailable')}
      disabled={arabic.length === 0}
      onchange={(arabicLayout) => save({ arabicLayout })}
    />
  </FormRow>
  <FormRow icon="keyboard" title={t('settings.layouts.latinLayout')} labelFor="latin-layout">
    <Select
      id="latin-layout"
      value={settings.latinLayout || latin[0]?.id || ''}
      options={options(latin)}
      placeholder={t('settings.layouts.unavailable')}
      disabled={latin.length === 0}
      onchange={(latinLayout) => save({ latinLayout })}
    />
  </FormRow>
</GroupCard>

{#if map && !missingArabic}
  <GroupCard title={t('settings.layouts.keyboardMap.title')} sunken>
    {#snippet trailing()}
      <bdi class="pair">{pairName}</bdi>
    {/snippet}
    <div class="map">
      <div class="keys">
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
        <p class="note">
          <Icon name="info" size={16} />
          {t('settings.layouts.keyboardMap.multiCharKeyCaption')}
        </p>
      {/if}
    </div>
  </GroupCard>
{/if}

<style>
  .pair {
    color: var(--text-tertiary);
    font-size: var(--size-caption);
    line-height: var(--leading-small);
    white-space: pre;
  }

  .map {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
  }

  .keys {
    display: flex;
    flex-direction: column;
    gap: 4px;
    direction: ltr;
  }

  .row {
    display: flex;
    justify-content: center;
    gap: 4px;
  }

  .key {
    position: relative;
    width: 36px;
    height: 38px;
    flex: none;
    border: 1px solid var(--keycap-edge);
    border-bottom-width: 2px;
    border-radius: var(--radius-control);
    background: var(--keycap-top);
  }

  .multi {
    border-color: var(--accent-primary);
    background: var(--accent-soft);
  }

  .latin {
    position: absolute;
    top: 3px;
    left: 4px;
    font-family: var(--font-latin);
    font-size: 10px;
    font-weight: 500;
    line-height: 12px;
    color: var(--text-tertiary);
  }

  .arabic {
    position: absolute;
    right: 4px;
    bottom: 1px;
    font-size: var(--size-body);
    line-height: var(--leading-body);
    color: var(--text-primary);
  }

  .multi .latin,
  .multi .arabic {
    color: var(--accent-primary);
  }

  .note {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    padding: 8px 12px;
    border-radius: var(--radius-tab);
    background: var(--accent-soft);
    color: var(--accent-primary);
    font-size: var(--size-small);
    line-height: var(--leading-small);
  }

  .alert {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 60px;
    padding: 12px 16px;
    border: 1px solid color-mix(in srgb, var(--state-warning) 45%, transparent);
    border-radius: var(--radius-card);
    background: var(--state-warning-soft);
  }

  .warn {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    flex: none;
    border-radius: 50%;
    background: var(--bg-surface);
    color: var(--state-warning);
  }

  .text {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
  }

  .text strong {
    font-size: var(--size-body);
    line-height: var(--leading-body);
  }

  .text span {
    color: var(--state-warning-text);
    font-size: var(--size-small);
    line-height: var(--leading-small);
  }
</style>
