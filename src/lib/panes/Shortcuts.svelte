<script lang="ts">
  import Button from '../components/Button.svelte'
  import FormRow from '../components/FormRow.svelte'
  import GroupCard from '../components/GroupCard.svelte'
  import ShortcutRecorder from '../components/ShortcutRecorder.svelte'
  import { app, setShortcut, t, type Binding, type Settings } from '../state.svelte'

  const settings = $derived(app.settings as Settings)

  /** Which row, if any, was last refused by the system. */
  let conflict = $state<Binding | null>(null)

  /** Keep in step with `Settings::default` in `src-tauri/src/settings.rs`. */
  const rows: { binding: Binding; key: keyof Settings; fallback: string }[] = [
    { binding: 'convert', key: 'shortcutConvert', fallback: 'Alt+Shift+Space' },
    { binding: 'undo', key: 'shortcutUndo', fallback: '' },
    { binding: 'pause', key: 'shortcutPause', fallback: '' },
  ]

  const isDefault = $derived(rows.every((row) => settings[row.key] === row.fallback))

  async function record(binding: Binding, accelerator: string) {
    conflict = (await setShortcut(binding, accelerator)) ? binding : null
  }

  async function restoreDefaults() {
    conflict = null
    for (const row of rows) {
      if (settings[row.key] !== row.fallback) await setShortcut(row.binding, row.fallback)
    }
  }
</script>

<GroupCard title={t('settings.shortcuts.groupTitle')} footnote={t('settings.shortcuts.footnote')}>
  {#each rows as row, index (row.binding)}
    <FormRow
      first={index === 0}
      title={t(`settings.shortcuts.rows.${row.binding}`)}
      description={conflict === row.binding ? t('shortcut.recorder.conflict') : undefined}
      tone="warning"
    >
      <ShortcutRecorder
        value={settings[row.key] as string}
        conflict={conflict === row.binding}
        label={t(`settings.shortcuts.rows.${row.binding}`)}
        onrecord={(accelerator) => record(row.binding, accelerator)}
      />
    </FormRow>
  {/each}
  {#snippet footAction()}
    <Button variant="plain" disabled={isDefault} onclick={restoreDefaults}>
      {t('settings.shortcuts.restoreDefaults')}
    </Button>
  {/snippet}
</GroupCard>
