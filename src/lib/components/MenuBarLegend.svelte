<script lang="ts">
  /**
   * `Welcome / Menu bar legend` in 02 — Components: the three faces of the menu bar icon in
   * a small bar of their own, each with one line saying what it means. The glyphs are the
   * real template images, drawn as a mask in the text colour so they follow the appearance
   * the way the system tints the real ones. The order follows the reading direction.
   */
  import { t } from '../state.svelte'
  import ready from '../../assets/menubar/ready.png'
  import paused from '../../assets/menubar/paused.png'
  import needs from '../../assets/menubar/needs-permission.png'

  const states = [
    { key: 'ready', glyph: ready },
    { key: 'paused', glyph: paused },
    { key: 'needs', glyph: needs },
  ] as const
</script>

<div class="legend">
  <div class="bar" aria-hidden="true">
    {#each states as state (state.key)}
      <span class="cell"><span class="glyph" style:--glyph="url({state.glyph})"></span></span>
    {/each}
  </div>
  <ul class="captions">
    {#each states as state (state.key)}
      <li>
        <span class="label">{t(`onboarding.how.menuBar.${state.key}.label`)}</span>
        <span class="body">{t(`onboarding.how.menuBar.${state.key}.body`)}</span>
      </li>
    {/each}
  </ul>
</div>

<style>
  /* Wider than the text column, like the three moments above it. */
  .legend {
    display: flex;
    flex: none;
    flex-direction: column;
    gap: var(--space-4);
    width: 496px;
  }

  .bar {
    display: flex;
    height: 24px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    background: var(--bg-surface-secondary);
    overflow: hidden;
  }

  .cell {
    display: grid;
    flex: 1;
    place-items: center;
  }

  /* 18pt, the size the real icon is drawn at. */
  .glyph {
    width: 18px;
    height: 18px;
    background: var(--text-primary);
    mask: var(--glyph) center / 18px 18px no-repeat;
    -webkit-mask: var(--glyph) center / 18px 18px no-repeat;
  }

  .captions {
    display: flex;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .captions li {
    display: flex;
    flex: 1;
    flex-direction: column;
    align-items: center;
    padding: 0 var(--space-4);
    text-align: center;
  }

  .label {
    font-size: var(--size-label);
    line-height: var(--leading-label);
    font-weight: 700;
  }

  .body {
    color: var(--text-secondary);
    font-size: var(--size-caption);
    line-height: var(--leading-caption);
  }
</style>
