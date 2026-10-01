<script lang="ts">
  import Icon from './Icon.svelte'
  import { t } from '../state.svelte'

  /**
   * `Diagnostics / Details` in 02 — Components: what "Copy diagnostic information"
   * copies, what it never copies, and that every report carries the same block.
   * Keep the list in step with `src-tauri/src/diagnostics.rs`.
   */
  const INCLUDED = ['versions', 'permission', 'layouts', 'history', 'frontmost', 'settings'] as const
</script>

<div class="details">
  <strong>{t('diagnostics.includes')}</strong>
  <ul>
    {#each INCLUDED as item (item)}
      <li><span class="check"><Icon name="check" size={12} /></span>{t(`diagnostics.items.${item}`)}</li>
    {/each}
  </ul>
  <p class="never"><span class="muted"><Icon name="noEye" size={12} /></span>{t('diagnostics.never')}</p>
  <hr />
  <p class="link"><span><Icon name="info" size={12} /></span>{t('diagnostics.sameInReports')}</p>
</div>

<style>
  .details {
    display: flex;
    flex-direction: column;
    gap: var(--space-8);
    padding: var(--space-12);
    border-radius: var(--radius-md);
    background: var(--bg-surface-secondary);
    font-size: var(--size-label);
    line-height: var(--leading-label);
    text-align: start;
  }

  strong {
    color: var(--text-secondary);
    font-weight: 700;
  }

  ul {
    display: flex;
    flex-direction: column;
    gap: var(--space-8);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li,
  p {
    display: flex;
    align-items: center;
    gap: var(--space-8);
  }

  span {
    display: grid;
    flex: none;
  }

  .check {
    color: var(--success);
  }

  .never,
  .muted {
    color: var(--text-secondary);
  }

  hr {
    width: 100%;
    height: 1px;
    margin: 0;
    border: none;
    background: var(--border-subtle);
  }

  .link {
    color: var(--text-brand);
  }
</style>
