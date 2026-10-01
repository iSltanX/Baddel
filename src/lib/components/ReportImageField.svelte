<script lang="ts">
  import Button from './Button.svelte'
  import Icon from './Icon.svelte'
  import Keycap from './Keycap.svelte'
  import { t, type ReportImage } from '../state.svelte'

  /**
   * `Report / Image` in 02 — Components: the optional image, empty or attached, with the
   * privacy warning once attached and the error line when an attempt fails.
   */
  interface Props {
    image: ReportImage | null
    error: string | null
    onchoose: () => void
    onremove: () => void
  }

  const { image, error, onchoose, onremove }: Props = $props()

  function size(bytes: number): string {
    return bytes < 1024 * 1024 ? `${Math.max(1, Math.round(bytes / 1024))} KB` : `${(bytes / 1024 / 1024).toFixed(1)} MB`
  }

  const meta = $derived(
    image ? [image.mime === 'image/jpeg' ? 'JPEG' : 'PNG', size(image.bytes), `${image.width}×${image.height}`].join(' · ') : '',
  )
</script>

<section class="group">
  <h2>{t('report.image')}</h2>
  <div class="card" class:error={!!error}>
    {#if image}
      <div class="row attached">
        <img class="thumb" src={image.thumbnail} alt="" />
        <div class="text">
          <span>{t('report.attached')}</span>
          <span class="meta ltr">{meta}</span>
        </div>
        <Button variant="plain" onclick={onremove}>{t('report.remove')}</Button>
      </div>
    {:else}
      <div class="row">
        <span class="paste">{t('report.pasteHint')}</span>
        <span class="keys ltr"><Keycap label="⌘" size="m" /><Keycap label="V" size="m" /></span>
        <span class="spacer"></span>
        <Button onclick={onchoose}>{t('report.chooseImage')}</Button>
      </div>
    {/if}
  </div>
  {#if error}
    <p class="line danger" role="alert"><Icon name="warning" size={16} />{error}</p>
  {:else if image}
    <p class="line warning"><Icon name="warning" size={16} />{t('report.imageWarning')}</p>
  {/if}
  <p class="limits">{t('report.imageLimits')}</p>
</section>

<style>
  .group {
    display: flex;
    flex-direction: column;
    gap: var(--space-8);
  }

  h2 {
    min-height: 22px;
    padding-inline: var(--space-16);
    font-family: var(--font-body);
    font-size: var(--size-label);
    line-height: 22px;
    font-weight: 700;
    color: var(--text-secondary);
  }

  .card {
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    background: var(--bg-surface);
    overflow: hidden;
  }

  .card.error {
    border-color: var(--danger);
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--space-8);
    min-height: 48px;
    padding: var(--space-8) var(--space-16);
  }

  .attached {
    gap: var(--space-12);
    min-height: 60px;
  }

  .paste {
    color: var(--text-secondary);
    font-size: var(--size-label);
    line-height: var(--leading-label);
  }

  .keys {
    display: inline-flex;
    gap: 3px;
  }

  .spacer {
    flex: 1;
  }

  .thumb {
    width: 56px;
    height: 40px;
    flex: none;
    object-fit: cover;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-xs);
    background: var(--bg-surface-secondary);
  }

  /* Each line hugs its text from the reading side, so the Latin meta line still starts
     where the Arabic one does. */
  .text {
    display: flex;
    flex: 1;
    flex-direction: column;
    align-items: flex-start;
    min-width: 0;
  }

  .meta {
    color: var(--text-secondary);
    font-family: var(--font-latin);
    font-size: var(--size-latin-small);
    line-height: var(--leading-label);
  }

  .line,
  .limits {
    padding-inline: var(--space-16);
    font-size: var(--size-label);
    line-height: var(--leading-label);
  }

  .line {
    display: flex;
    align-items: center;
    gap: var(--space-8);
  }

  .warning {
    color: var(--warning-text);
  }

  .warning :global(svg) {
    color: var(--warning);
  }

  .danger,
  .danger :global(svg) {
    color: var(--danger);
  }

  .limits {
    color: var(--text-secondary);
  }
</style>
