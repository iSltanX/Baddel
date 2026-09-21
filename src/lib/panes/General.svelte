<script lang="ts">
  import Button from '../components/Button.svelte'
  import FormRow from '../components/FormRow.svelte'
  import GroupCard from '../components/GroupCard.svelte'
  import PermissionCard from '../components/PermissionCard.svelte'
  import Select from '../components/Select.svelte'
  import Switch from '../components/Switch.svelte'
  import { app, appVersion, previewHud, save, t, type Settings } from '../state.svelte'

  const settings = $derived(app.settings as Settings)

  let version = $state('')
  $effect(() => {
    void appVersion().then((value) => (version = value))
  })
</script>

<!-- A missing permission is the one thing that stops the app working, so it leads the pane. -->
{#if !app.permission}
  <PermissionCard />
{/if}

<GroupCard title={t('settings.general.groups.startup')}>
  <FormRow first icon="power" title={t('settings.general.launchAtLogin')}>
    <Switch
      checked={settings.launchAtLogin}
      label={t('settings.general.launchAtLogin')}
      onchange={(launchAtLogin) => save({ launchAtLogin })}
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
    description={version ? t('settings.about.version', { version }) : undefined}
  >
    <!-- The updater itself is wired up in phase 5, with its signing key. -->
    <Button disabled>{t('settings.general.checkNow')}</Button>
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
    gap: 8px;
  }

  h2 {
    display: flex;
    align-items: center;
    min-height: 22px;
    padding-inline: 16px;
    font-family: var(--font-body);
    font-size: var(--size-small);
    line-height: var(--leading-small);
    font-weight: 700;
    color: var(--text-secondary);
  }
</style>
