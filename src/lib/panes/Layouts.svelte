<script lang="ts">
  import Badge from '../components/Badge.svelte'
  import Button from '../components/Button.svelte'
  import FormRow from '../components/FormRow.svelte'
  import GroupCard from '../components/GroupCard.svelte'
  import Icon from '../components/Icon.svelte'
  import Select from '../components/Select.svelte'
  import {
    app,
    convertText,
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

  // «جرّبها هنا» (ج4): the text goes through the very command the shortcut uses, with the layouts
  // in use, and lives in this field alone — nothing typed here is kept or sent anywhere.
  let sample = $state('')
  let converted = $state('')
  let asked = 0

  async function tryIt(text: string) {
    const ask = ++asked
    const result = text.trim() ? await convertText(text) : null
    // An older answer arriving late must not replace a newer one.
    if (ask === asked) converted = result ?? text
  }

  $effect(() => {
    // Converted again when the chosen layouts change, as the map is.
    void settings.arabicLayout
    void settings.latinLayout
    void tryIt(sample)
  })

  const arabicScript = (text: string) => /[\u0600-\u06FF\u0750-\u077F\uFB50-\uFDFF\uFE70-\uFEFF]/.test(text)
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

  <GroupCard title={t('settings.layouts.tryIt.title')}>
    <div class="try">
      <!-- Typed text on one side, its conversion on the other: a left-to-right row in both languages. -->
      <div class="try-row">
        <input
          class="try-field"
          class:ar-script={arabicScript(sample)}
          bind:value={sample}
          dir="auto"
          aria-label={t('settings.layouts.tryIt.label')}
          spellcheck="false"
          autocomplete="off"
        />
        <span class="try-arrow" aria-hidden="true">→</span>
        <bdi
          class="try-result"
          class:ar-script={arabicScript(converted)}
          aria-label={t('settings.layouts.tryIt.result')}
          aria-live="polite">{sample.trim() ? converted : ''}</bdi
        >
      </div>
      <p class="try-caption">{t('settings.layouts.tryIt.caption')}</p>
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

  /* `Group / جرّبها هنا` in Settings · Layouts. */
  .try {
    display: flex;
    flex-direction: column;
    gap: var(--space-12);
    padding: var(--space-12) var(--space-16);
  }

  .try-row {
    display: flex;
    align-items: center;
    gap: var(--space-12);
    min-height: 32px;
    direction: ltr;
  }

  /* The field keeps Figma's share of the row (334 of 484); the result has the rest. */
  .try-field {
    flex: 0 0 69%;
    min-width: 0;
    height: 32px;
    padding: var(--space-4) var(--space-8);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-sm);
    background: var(--bg-surface-secondary);
    color: var(--text-primary);
    font-family: var(--font-latin);
    font-size: var(--size-section);
    caret-color: var(--border-focus);
    user-select: text;
  }

  .try-field:focus-visible {
    border-color: var(--border-focus);
    outline: none;
    box-shadow: var(--focus-ring);
  }

  .try-arrow {
    flex: none;
    color: var(--text-tertiary);
    font-family: var(--font-latin);
    font-size: var(--size-section);
  }

  .try-result {
    flex: 1 1 0;
    min-width: 0;
    color: var(--text-brand);
    font-family: var(--font-latin);
    font-size: 22px;
    font-weight: 700;
    line-height: 32px;
    overflow-wrap: anywhere;
  }

  .try-field.ar-script,
  .try-result.ar-script {
    font-family: var(--font-body);
  }

  .try-caption {
    margin: 0;
    color: var(--text-secondary);
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
