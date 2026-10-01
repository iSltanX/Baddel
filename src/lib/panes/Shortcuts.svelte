<script lang="ts">
  import Button from '../components/Button.svelte'
  import FormRow from '../components/FormRow.svelte'
  import GroupCard from '../components/GroupCard.svelte'
  import ShortcutRecorder from '../components/ShortcutRecorder.svelte'
  import { app, setShortcut, t, type Binding, type Settings, type ShortcutConflict } from '../state.svelte'

  const settings = $derived(app.settings as Settings)

  /** Which row, if any, was last refused, and why. */
  let conflict = $state<{ row: Binding; reason: ShortcutConflict } | null>(null)

  type Row = { binding: Binding; key: keyof Settings; fallback: string }

  /** Keep in step with `Settings::default` in `src-tauri/src/settings.rs`. */
  const general: Row[] = [
    { binding: 'convert', key: 'shortcutConvert', fallback: 'Alt+Shift+Space' },
    { binding: 'undo', key: 'shortcutUndo', fallback: '' },
    { binding: 'pause', key: 'shortcutPause', fallback: '' },
  ]
  /** The optional commands: never a default shortcut. */
  const extra: Row[] = [
    { binding: 'convertLine', key: 'shortcutConvertLine', fallback: '' },
    { binding: 'toArabic', key: 'shortcutToArabic', fallback: '' },
    { binding: 'toLatin', key: 'shortcutToLatin', fallback: '' },
  ]
  const rows = [...general, ...extra]

  const isDefault = $derived(rows.every((row) => settings[row.key] === row.fallback))

  async function record(binding: Binding, accelerator: string) {
    const reason = await setShortcut(binding, accelerator)
    conflict = reason ? { row: binding, reason } : null
  }

  /** The refusal, in the row's own words: another app, or the Baddel command that has it. */
  function conflictMessage(reason: ShortcutConflict): string {
    if (!reason.with) return t('shortcut.recorder.conflict')
    return t('shortcut.recorder.conflictInternal', { command: t(`settings.shortcuts.rows.${reason.with}`) })
  }

  async function restoreDefaults() {
    conflict = null
    // Unbinding first, so a default is never refused for a shortcut another row still holds.
    const changed = rows.filter((row) => settings[row.key] !== row.fallback)
    for (const row of changed) if (settings[row.key] !== '') await setShortcut(row.binding, '')
    for (const row of changed) if (row.fallback !== '') await setShortcut(row.binding, row.fallback)
  }
</script>

{#snippet recorderRows(group: Row[])}
  {#each group as row, index (row.binding)}
    <FormRow
      first={index === 0}
      title={t(`settings.shortcuts.rows.${row.binding}`)}
      description={conflict?.row === row.binding ? conflictMessage(conflict.reason) : undefined}
      tone="warning"
    >
      <ShortcutRecorder
        value={settings[row.key] as string}
        conflict={conflict?.row === row.binding}
        label={t(`settings.shortcuts.rows.${row.binding}`)}
        onrecord={(accelerator) => record(row.binding, accelerator)}
      />
    </FormRow>
  {/each}
{/snippet}

<GroupCard title={t('settings.shortcuts.groupTitle')}>
  {@render recorderRows(general)}
</GroupCard>

<!-- `Settings / الاختصارات · ستة أوامر`: the optional commands in a group of their own. -->
<GroupCard title={t('settings.shortcuts.extraGroupTitle')} footnote={t('settings.shortcuts.footnote')}>
  {@render recorderRows(extra)}
  {#snippet footAction()}
    <Button variant="plain" disabled={isDefault} onclick={restoreDefaults}>
      {t('settings.shortcuts.restoreDefaults')}
    </Button>
  {/snippet}
</GroupCard>
