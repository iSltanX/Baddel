<script lang="ts">
  import { t } from '../state.svelte'

  /**
   * `Report / Description` in 02 — Components: the required description, with its hint
   * and a counter that turns to the danger colour past the limit.
   */
  interface Props {
    value: string
    max: number
    disabled?: boolean
  }

  let { value = $bindable(), max, disabled = false }: Props = $props()

  const count = $derived([...value.trim()].length)
  const over = $derived(count > max)
</script>

<section class="group">
  <h2><label for="description">{t('report.description')}</label></h2>
  <textarea
    id="description"
    class:over
    dir={value ? 'auto' : undefined}
    rows="5"
    maxlength={max + 200}
    placeholder={t('report.descriptionPlaceholder')}
    aria-invalid={over}
    {disabled}
    bind:value
  ></textarea>
  <div class="note">
    <p>{t('report.descriptionHint')}</p>
    <span class="counter" class:over>{t('report.counter', { count, max })}</span>
  </div>
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

  textarea {
    display: block;
    width: 100%;
    min-height: 120px;
    padding: var(--space-8) var(--space-12);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-sm);
    background: var(--bg-surface);
    font-size: var(--size-body);
    line-height: var(--leading-body);
    resize: none;
    user-select: text;
    cursor: text;
    transition: border-color 120ms ease-out;
  }

  textarea::placeholder {
    color: var(--text-tertiary);
  }

  textarea:hover:not(:disabled) {
    border-color: var(--text-tertiary);
  }

  textarea:focus-visible {
    border-color: var(--border-focus);
    box-shadow: var(--focus-ring);
  }

  textarea.over,
  textarea.over:hover {
    border-color: var(--danger);
  }

  textarea:disabled {
    opacity: 0.45;
  }

  .note {
    display: flex;
    align-items: baseline;
    gap: var(--space-12);
    padding-inline: var(--space-16);
    color: var(--text-secondary);
    font-size: var(--size-label);
    line-height: var(--leading-label);
  }

  .note p {
    flex: 1;
  }

  .counter {
    flex: none;
    color: var(--text-tertiary);
  }

  .counter.over {
    color: var(--danger);
  }
</style>
