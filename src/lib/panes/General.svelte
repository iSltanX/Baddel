<script lang="ts">
  import Button from '../components/Button.svelte'
  import FormRow from '../components/FormRow.svelte'
  import GroupCard from '../components/GroupCard.svelte'
  import PermissionCard from '../components/PermissionCard.svelte'
  import Select from '../components/Select.svelte'
  import Switch from '../components/Switch.svelte'
  import {
    app,
    appVersion,
    checkForUpdates,
    installUpdate,
    language,
    openLoginItems,
    previewHud,
    save,
    setLaunchAtLogin,
    t,
    type Settings,
  } from '../state.svelte'

  const settings = $derived(app.settings as Settings)

  // Launch at login shows what macOS has, not what was last saved (`startup.rs`).
  const startup = $derived(app.startup)
  const launchesAtLogin = $derived(startup.state === 'enabled' || startup.state === 'requiresApproval')
  let switching = $state(false)
  const startupLine = $derived.by(() => {
    const failure = startup.failure
    if (failure) return t(`settings.general.startup.problem.${failure.problem}`, { code: failure.code ?? '—' })
    if (startup.state === 'requiresApproval') return t('settings.general.startup.requiresApproval')
    if (startup.legacy === 'unverified') return t('settings.general.startup.legacyUnverified')
    return undefined
  })
  /** The line is a problem to fix, rather than a note. */
  const startupWarns = $derived(!!startup.failure || startup.state === 'requiresApproval')
  /** Where the user can approve, or remove what Baddel could not. */
  const offersLoginItems = $derived(
    startup.state === 'requiresApproval' ||
      startup.failure?.problem === 'deniedByUser' ||
      startup.failure?.problem === 'stillEnabled',
  )

  async function switchLaunchAtLogin(enabled: boolean) {
    switching = true
    try {
      await setLaunchAtLogin(enabled)
    } finally {
      switching = false
    }
  }

  let version = $state('')
  $effect(() => {
    void appVersion().then((value) => (version = value))
  })

  const update = $derived(app.update)
  const busy = $derived(update.phase === 'checking' || update.phase === 'installing')

  function checkedAt(ms: number): string {
    // Latin digits, matching the version number beside it.
    const locale = language() === 'ar' ? 'ar-u-nu-latn' : 'en'
    return new Intl.DateTimeFormat(locale, { dateStyle: 'medium', timeStyle: 'short' }).format(ms)
  }

  const updateLine = $derived.by(() => {
    if (!version) return undefined
    switch (update.phase) {
      case 'checking':
        return t('settings.general.update.checking')
      case 'available':
        return t('settings.general.update.available', { version: update.version ?? '' })
      case 'installing':
        return t('settings.general.update.installing', { version: update.version ?? '' })
      case 'failed':
        // A failed install names the version that did not install, and the one still running.
        if (update.failure === 'download') return t('settings.general.update.downloadFailed', { version: update.version ?? '' })
        if (update.failure === 'install') return t('settings.general.update.installFailed', { version: update.version ?? '', current: version })
        if (update.failure === 'restore') return t('settings.general.update.restoreFailed', { current: version })
        return t('settings.general.update.failed', { version })
      default:
        return update.lastChecked
          ? t('settings.general.update.lastChecked', { version, time: checkedAt(update.lastChecked) })
          : t('settings.about.version', { version })
    }
  })
</script>

<!-- A missing permission is the one thing that stops the app working, so it leads the pane. -->
{#if !app.permission}
  <PermissionCard />
{/if}

<GroupCard title={t('settings.general.groups.startup')}>
  <FormRow
    first
    icon="power"
    title={t('settings.general.launchAtLogin')}
    description={startupLine}
    tone={startupWarns ? 'warning' : 'default'}
  >
    {#if offersLoginItems}
      <Button onclick={() => void openLoginItems()}>{t('settings.general.startup.openLoginItems')}</Button>
    {/if}
    <Switch
      checked={launchesAtLogin}
      disabled={switching}
      label={t('settings.general.launchAtLogin')}
      onchange={(enabled) => void switchLaunchAtLogin(enabled)}
    />
  </FormRow>
  <FormRow
    icon="menubar"
    title={t('settings.general.showMenuBarIcon')}
    description={t('settings.general.showMenuBarIconDescription')}
  >
    <Switch
      checked={settings.showTrayIcon}
      label={t('settings.general.showMenuBarIcon')}
      onchange={(showTrayIcon) => save({ showTrayIcon })}
    />
  </FormRow>
  <FormRow icon="globe" title={t('settings.general.uiLanguage')} labelFor="ui-language">
    <Select
      id="ui-language"
      value={settings.language}
      options={[
        { value: 'ar', label: t('settings.general.uiLanguageOptions.ar') },
        { value: 'en', label: t('settings.general.uiLanguageOptions.en') },
      ]}
      onchange={(language) => save({ language: language as 'ar' | 'en' })}
    />
  </FormRow>
</GroupCard>

<GroupCard title={t('settings.general.groups.conversion')}>
  <FormRow
    first
    icon="swap"
    title={t('settings.general.switchLayoutAfterConvert')}
    description={t('settings.general.switchLayoutAfterConvertDescription')}
  >
    <Switch
      checked={settings.switchInputSource}
      label={t('settings.general.switchLayoutAfterConvert')}
      onchange={(switchInputSource) => save({ switchInputSource })}
    />
  </FormRow>
  <FormRow icon="notice" title={t('settings.general.showConversionNotification')}>
    <Button disabled={!settings.showHud} onclick={previewHud}>
      {t('settings.general.previewNotification')}
    </Button>
    <Switch
      checked={settings.showHud}
      label={t('settings.general.showConversionNotification')}
      onchange={(showHud) => save({ showHud })}
    />
  </FormRow>
  <FormRow icon="speaker" title={t('settings.general.softSoundOnConvert')}>
    <Switch
      checked={settings.sound}
      label={t('settings.general.softSoundOnConvert')}
      onchange={(sound) => save({ sound })}
    />
  </FormRow>
</GroupCard>

<GroupCard title={t('settings.general.groups.updates')}>
  <FormRow
    first
    icon="refresh"
    title={t('settings.general.autoCheckUpdates')}
    description={updateLine}
  >
    {#if update.phase === 'available' || update.phase === 'installing' || (update.phase === 'failed' && (update.failure === 'download' || update.failure === 'install'))}
      <Button variant="primary" loading={update.phase === 'installing'} onclick={() => void installUpdate()}>
        {t('settings.general.update.install')}
      </Button>
    {:else}
      <Button loading={busy} onclick={() => void checkForUpdates()}>{t('settings.general.checkNow')}</Button>
    {/if}
    <Switch
      checked={settings.autoUpdate}
      label={t('settings.general.autoCheckUpdates')}
      onchange={(autoUpdate) => save({ autoUpdate })}
    />
  </FormRow>
</GroupCard>

<!-- Once granted it is only a reassurance, so it closes the pane quietly. -->
{#if app.permission}
  <section class="permission">
    <h2>{t('settings.general.groups.permission')}</h2>
    <PermissionCard />
  </section>
{/if}

<style>
  .permission {
    display: flex;
    flex-direction: column;
    gap: var(--space-8);
  }

  h2 {
    display: flex;
    align-items: center;
    min-height: 18px;
    padding-inline: var(--space-16);
    font-family: var(--font-body);
    font-size: var(--size-label);
    line-height: var(--leading-label);
    font-weight: 700;
    color: var(--text-secondary);
  }
</style>
