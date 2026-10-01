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
    font-family: var(--font-latin);
    font-size: var(--size-latin-small);
    line-height: var(--leading-label);
    white-space: pre;
  }

  .map {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-12);
    padding: var(--space-16);
  }

  .keys {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    direction: ltr;
  }

  .row {
    display: flex;
    justify-content: center;
    gap: var(--space-4);
  }

  /* `Key` in 02 — Components. */
  .key {
    position: relative;
    width: 36px;
    height: 38px;
    flex: none;
    border: 1px solid var(--keycap-edge);
    border-bottom-width: 2px;
    border-radius: var(--radius-sm);
    background: var(--keycap-top);
  }

  .multi {
    border-color: var(--action-primary);
    background: var(--bg-selected);
  }

  .latin {
    position: absolute;
    top: 3px;
    left: 4px;
    font-family: var(--font-latin);
    font-size: var(--size-keycap-small);
    font-weight: 500;
    line-height: var(--leading-keycap-small);
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
    color: var(--text-brand);
  }

  .note {
    display: flex;
    align-items: center;
    gap: var(--space-8);
    align-self: stretch;
    padding: var(--space-8) var(--space-12);
    border-radius: var(--radius-md);
    background: var(--bg-selected);
    color: var(--text-brand);
    font-size: var(--size-label);
    line-height: var(--leading-label);
  }

  .alert {
    display: flex;
    align-items: center;
    gap: var(--space-12);
    padding: var(--space-12) var(--space-16);
    border: 1px solid var(--warning);
    border-radius: var(--radius-md);
    background: var(--warning-soft);
  }

  .warn {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    flex: none;
    border-radius: var(--radius-full);
    background: var(--bg-surface);
    color: var(--warning);
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
    color: var(--warning-text);
    font-size: var(--size-label);
    line-height: var(--leading-label);
  }
</style>
