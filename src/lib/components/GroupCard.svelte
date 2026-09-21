<script lang="ts">
  import type { Snippet } from 'svelte'

  /**
   * A grouped form card: a quiet label, an inset surface holding one to six rows,
   * and an optional footnote. The label, the rows' content and the footnote all
   * start on the same 16px line.
   */
  interface Props {
    title?: string
    /** A badge or caption on the far side of the label row. */
    trailing?: Snippet
    footnote?: string
    /** An action that sits beside the footnote, on the far side. */
    footAction?: Snippet
    /** The sunken variant holds objects (the keyboard), not rows. */
    sunken?: boolean
    children: Snippet
  }

  const { title, trailing, footnote, footAction, sunken = false, children }: Props = $props()
</script>

<section>
  {#if title || trailing}
    <header>
      <h2>{title ?? ''}</h2>
      {#if trailing}{@render trailing()}{/if}
    </header>
  {/if}
  <div class="card" class:sunken>
    {@render children()}
  </div>
  {#if footnote || footAction}
    <footer>
      <p>{footnote ?? ''}</p>
      {#if footAction}{@render footAction()}{/if}
    </footer>
  {/if}
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  header,
  footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding-inline: 16px;
  }

  header {
    min-height: 22px;
  }

  h2 {
    flex: 1;
    font-family: var(--font-body);
    font-size: var(--size-small);
    line-height: var(--leading-small);
    font-weight: 700;
    color: var(--text-secondary);
  }

  footer {
    gap: 12px;
  }

  p {
    flex: 1;
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--size-small);
    line-height: var(--leading-small);
  }

  .card {
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-card);
    background: var(--bg-surface);
    overflow: hidden;
  }

  .sunken {
    background: var(--bg-sunken);
  }
</style>
