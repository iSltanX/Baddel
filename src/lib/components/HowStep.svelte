<script lang="ts">
  import Keycap from './Keycap.svelte'

  /**
   * `Welcome / How step` in 02 — Components: one of the three moments of a conversion —
   * the word as typed, the shortcut, the word fixed with the notice confirming it. The
   * pictures are LTR objects (a text field, a keyboard) and are never mirrored.
   */
  interface Props {
    stage: 'type' | 'press' | 'fixed'
    number: number
    label: string
    /** The word as typed, and as fixed. */
    from: string
    to: string
    /** The user's real shortcut, as keycap labels. */
    keys: string[]
  }

  const { stage, number, label, from, to, keys }: Props = $props()
</script>

<div class="step">
  <div class="picture ltr {stage}">
    {#if stage === 'type'}
      <span class="typed"><span class="caret" aria-hidden="true"></span><bdi>{from}</bdi></span>
    {:else if stage === 'press'}
      <span class="keys">
        {#each keys as key (key)}<Keycap label={key} />{/each}
      </span>
    {:else}
      <bdi class="fixed-word">{to}</bdi>
      <span class="mini-hud" aria-hidden="true"><span class="dot"></span>EN</span>
    {/if}
  </div>
  <div class="caption">
    <span class="number" class:brand={stage === 'fixed'}>{number}</span>
    <span class="label">{label}</span>
  </div>
</div>

<style>
  .step {
    display: flex;
    flex-direction: column;
    gap: var(--space-12);
    width: 148px;
    height: 168px;
    flex: none;
    padding: var(--space-12);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    background: var(--bg-surface);
  }

  .picture {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-12);
    height: 96px;
    flex: none;
    border-radius: var(--radius-md);
    background: var(--bg-surface-secondary);
  }

  .picture.fixed {
    background: var(--bg-selected);
  }

  /* The caret sits where typing stopped: after the word, which for Arabic is its left end. */
  .typed {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    font-size: 20px;
    line-height: 22px;
  }

  .caret {
    width: 2px;
    height: 24px;
    background: var(--border-focus);
  }

  /* The small keycap drawn 1.2 times larger, as the picture draws it: a 24pt key with the
     small cap's proportions, so three keys fit across. */
  .keys {
    display: flex;
    gap: var(--space-4);
    zoom: 1.2;
  }

  .fixed-word {
    color: var(--text-brand);
    font-family: var(--font-latin);
    font-size: 22px;
    font-weight: 600;
    line-height: 26px;
  }

  /* The notice in miniature: the same ink pill as the real one. */
  .mini-hud {
    display: inline-flex;
    align-items: center;
    gap: var(--space-4);
    padding: 3px var(--space-8);
    border-radius: var(--radius-full);
    background: var(--hud-bg);
    color: var(--hud-text);
    font-family: var(--font-latin);
    font-size: 10px;
    font-weight: 500;
    line-height: 12px;
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--hud-success);
  }

  /* The label may take the card's end padding: the Arabic labels then hold one line, as the
     design sets them, and the longer English ones wrap to a second. */
  .caption {
    display: flex;
    align-items: center;
    gap: var(--space-8);
    margin-inline-end: calc(-1 * var(--space-8));
  }

  .number {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    flex: none;
    border-radius: 50%;
    background: var(--bg-surface-secondary);
    color: var(--text-secondary);
    font-family: var(--font-latin);
    font-size: 11px;
    font-weight: 700;
  }

  .number.brand {
    background: var(--action-primary);
    color: var(--text-on-primary);
  }

  .label {
    font-size: var(--size-label);
    line-height: var(--leading-label);
    font-weight: 700;
  }
</style>
