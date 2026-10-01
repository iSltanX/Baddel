<script lang="ts">
  import Icon from './Icon.svelte'
  import Keycap from './Keycap.svelte'
  import { fromEvent, toGlyphs } from '../accelerator'
  import { t } from '../state.svelte'

  /**
   * `Recorder` in 02 — Components: the shortcut field. It is as wide as a pop-up, so the control column stays a
   * column. The keys start where the field starts, and the clear button lives
   * inside the field on the far side, shown on hover or focus. A conflict is
   * explained by the row that owns the recorder, not by the recorder itself.
   */
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
  const filled = $derived(glyphs.length > 0 && !recording)

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

<div class="recorder" class:recording class:conflict={conflict && !recording} class:filled class:disabled>
  <button
    type="button"
    class="field"
    {disabled}
    aria-label={label}
    onclick={() => (recording = !recording)}
    onkeydown={keydown}
    onblur={() => (recording = false)}
  >
    {#if recording}
      <span class="hint">{t('shortcut.recorder.recording')}</span>
    {:else if filled}
      <span class="keys">
        {#each glyphs as glyph (glyph)}
          <Keycap label={glyph} />
        {/each}
      </span>
    {:else}
      <span class="hint">{t('shortcut.recorder.empty')}</span>
    {/if}
  </button>
  {#if filled && conflict}
    <span class="trailing warn"><Icon name="warning" size={16} /></span>
  {:else if filled}
    <button
      type="button"
      class="trailing clear"
      aria-label="{t('common.remove')} — {label}"
      {disabled}
      onclick={() => onrecord('')}
    >
      <Icon name="close" size={12} />
    </button>
  {/if}
</div>

<style>
  .recorder {
    position: relative;
    display: inline-flex;
    width: var(--control-column);
    flex: none;
  }

  .field {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: var(--control-md);
    padding: 0 var(--space-4);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    background: var(--bg-surface-secondary);
    transition: border-color 120ms ease-out;
  }

  /* With keys in it, the field reads from its start like any other value. */
  .filled .field {
    justify-content: flex-start;
  }

  .recorder:hover .field:not(:disabled) {
    border-color: var(--border-default);
  }

  .recording .field,
  .recording:hover .field {
    border: 1.5px solid var(--action-primary);
    background: var(--bg-surface);
    box-shadow: var(--focus-ring);
  }

  .field:focus-visible {
    box-shadow: var(--focus-ring);
  }

  .recording .hint {
    color: var(--text-brand);
  }

  .conflict .field,
  .conflict:hover .field {
    border-color: var(--warning);
    background: var(--warning-soft);
  }

  .disabled {
    opacity: 0.4;
  }

  /* A shortcut is an LTR run wherever it appears: ⌥ ⇧ Space, never mirrored. */
  .keys {
    display: flex;
    gap: 3px;
    direction: ltr;
  }

  .hint {
    font-size: var(--size-label);
    line-height: var(--leading-label);
    color: var(--text-secondary);
  }

  .trailing {
    position: absolute;
    inset-block: 0;
    inset-inline-end: var(--space-4);
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    margin-block: auto;
  }

  .warn {
    color: var(--warning);
  }

  /* Always in the tab order; visible on hover or focus, as macOS does it. */
  .clear {
    border: none;
    border-radius: var(--radius-full);
    background: var(--overlay-pressed);
    color: var(--text-secondary);
    opacity: 0;
    transition: opacity 120ms ease-out;
  }

  .recorder:hover .clear,
  .clear:focus-visible {
    opacity: 1;
  }

  .clear:focus-visible {
    box-shadow: var(--focus-ring);
  }
</style>
