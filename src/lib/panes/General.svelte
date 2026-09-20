<script lang="ts">
  import Button from '../components/Button.svelte'
  import FormRow from '../components/FormRow.svelte'
  import GroupCard from '../components/GroupCard.svelte'
  import PermissionCard from '../components/PermissionCard.svelte'
  import Select from '../components/Select.svelte'
  import Switch from '../components/Switch.svelte'
  import { app, previewHud, save, t, type Settings } from '../state.svelte'

  const settings = $derived(app.settings as Settings)
</script>

<GroupCard>
  <FormRow first icon="power" title={t('settings.general.launchAtLogin')}>
    <Switch
      checked={settings.launchAtLogin}
      label={t('settings.general.launchAtLogin')}
      onchange={(launchAtLogin) => save({ launchAtLogin })}
    />
  </FormRow>
  <FormRow
    icon="sidebar"
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

<GroupCard>
  <FormRow first icon="swap" title={t('settings.general.switchLayoutAfterConvert')}>
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

<GroupCard>
  <FormRow first icon="refresh" title={t('settings.general.autoCheckUpdates')}>
    <!-- The updater itself is wired up in phase 5, with its signing key. -->
    <Button disabled>{t('settings.general.checkNow')}</Button>
    <Switch
      checked={settings.autoUpdate}
      label={t('settings.general.autoCheckUpdates')}
      onchange={(autoUpdate) => save({ autoUpdate })}
    />
  </FormRow>
</GroupCard>

<PermissionCard />
