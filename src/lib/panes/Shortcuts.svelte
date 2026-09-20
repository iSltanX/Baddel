<script lang="ts">
  import FormRow from '../components/FormRow.svelte'
  import GroupCard from '../components/GroupCard.svelte'
  import ShortcutRecorder from '../components/ShortcutRecorder.svelte'
  import { app, setShortcut, t, type Binding, type Settings } from '../state.svelte'

  const settings = $derived(app.settings as Settings)

  /** Which row, if any, was last refused by the system. */
  let conflict = $state<Binding | null>(null)

  const rows: { binding: Binding; key: keyof Settings }[] = [
    { binding: 'convert', key: 'shortcutConvert' },
    { binding: 'undo', key: 'shortcutUndo' },
    { binding: 'pause', key: 'shortcutPause' },
  ]

  async function record(binding: Binding, accelerator: string) {
    conflict = (await setShortcut(binding, accelerator)) ? binding : null
  }
</script>

<GroupCard>
  {#each rows as row, index (row.binding)}
    <FormRow first={index === 0} icon="keyboard" title={t(`settings.shortcuts.rows.${row.binding}`)}>
      <ShortcutRecorder
        value={settings[row.key] as string}
        conflict={conflict === row.binding}
        label={t(`settings.shortcuts.rows.${row.binding}`)}
        onrecord={(accelerator) => record(row.binding, accelerator)}
      />
    </FormRow>
  {/each}
</GroupCard>

<p class="footnote">{t('settings.shortcuts.footnote')}</p>

<style>
  .footnote {
    margin: 0;
    padding-inline: 4px;
    color: var(--text-secondary);
    font-size: var(--size-small);
    line-height: var(--leading-small);
  }
</style>
