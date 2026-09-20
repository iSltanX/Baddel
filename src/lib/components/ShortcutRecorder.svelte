<script lang="ts">
  import Icon from './Icon.svelte'
  import Keycap from './Keycap.svelte'
  import { fromEvent, toGlyphs } from '../accelerator'
  import { t } from '../state.svelte'

  interface Props {
    value: string
    /** Set when the last attempt was refused because another app holds it. */
    conflict?: boolean
    disabled?: boolean
    label: string
    onrecord: (accelerator: string) => void
  }

  const { value, conflict = false, disabled = false, label, onrecord }: Props = $props()

  let recording = $state(false)
  const glyphs = $derived(toGlyphs(value))

  function keydown(event: KeyboardEvent) {
    if (!recording) return
    event.preventDefault()
    if (event.code === 'Escape') {
      recording = false
      return
    }
    const accelerator = fromEvent(event)
    if (!accelerator) return
    recording = false
    onrecord(accelerator)
  }
</script>

<div class="recorder">
  <button
    type="button"
    class="field"
    class:recording
    class:conflict
    {disabled}
    aria-label={label}
    onclick={() => (recording = !recording)}
    onkeydown={keydown}
    onblur={() => (recording = false)}
  >
    {#if recording}
      <span class="hint">{t('shortcut.recorder.recording')}</span>
    {:else if glyphs.length > 0}
      <span class="keys">
        {#each glyphs as glyph (glyph)}
          <Keycap label={glyph} />
        {/each}
      </span>
    {:else}
      <span class="hint">{t('shortcut.recorder.empty')}</span>
    {/if}
  </button>
  {#if value && !recording}
    <button
      type="button"
      class="clear"
      aria-label="{t('common.remove')} — {label}"
      {disabled}
      onclick={() => onrecord('')}
    >
      <Icon name="close" size={13} />
    </button>
  {/if}
</div>

{#if conflict}
  <p class="conflict-note" role="alert">
    <Icon name="warning" size={14} />
    {t('shortcut.recorder.conflict')}
  </p>
{/if}

<style>
  .recorder {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .field {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    min-width: 208px;
    height: 30px;
    padding: 0 10px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-control);
    background: var(--bg-sunken);
  }

  .field:hover:not(:disabled) {
    background: var(--overlay-hover);
  }

  .recording {
    outline: var(--focus-width) solid var(--accent-primary);
    outline-offset: var(--focus-offset);
    background: var(--accent-soft);
  }

  .conflict {
    border-color: var(--state-warning);
  }

  .field:disabled {
    opacity: 0.38;
  }

  /* A shortcut is an LTR run wherever it appears: ⌥ ⇧ Space, never mirrored. */
  .keys {
    display: flex;
    gap: 4px;
    direction: ltr;
  }

  .hint {
    font-size: var(--size-small);
    color: var(--text-secondary);
  }

  .clear {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border: none;
    border-radius: var(--radius-control);
    background: none;
    color: var(--text-secondary);
  }

  .clear:hover {
    background: var(--overlay-hover);
  }

  .conflict-note {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 6px 16px 0;
    color: var(--state-warning);
    font-size: var(--size-small);
    line-height: var(--leading-small);
  }
</style>
