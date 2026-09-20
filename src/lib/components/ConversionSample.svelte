<script lang="ts">
  /**
   * "اثممخ ← hello". Each side is its own bidi run, so an Arabic and a Latin word
   * sit side by side without either dragging the other around; the arrow points
   * the way the interface reads.
   */
  import { direction } from '../state.svelte'

  interface Props {
    from: string
    to: string
    size?: 'inline' | 'hero'
  }

  const { from, to, size = 'inline' }: Props = $props()
  const arrow = $derived(direction() === 'rtl' ? '←' : '→')
</script>

<span class="sample {size}">
  <bdi class="from">{from}</bdi>
  <span class="arrow" aria-hidden="true">{arrow}</span>
  <bdi class="to">{to}</bdi>
</span>

<style>
  .sample {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: var(--size-body);
    line-height: var(--leading-body);
  }

  .hero {
    font-size: 20px;
    line-height: 30px;
    font-weight: 700;
  }

  .from {
    color: var(--text-secondary);
  }

  .to {
    color: var(--accent-primary);
    font-weight: 700;
  }

  .arrow {
    color: var(--text-secondary);
  }
</style>
