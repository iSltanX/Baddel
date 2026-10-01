<script lang="ts">
  import type { Snippet } from 'svelte'

  /**
   * A grouped form card (the `Group` pattern in 03 — Product UI): a quiet label, a
   * surface holding one to six rows,
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
    gap: var(--space-8);
  }

  header,
  footer {
    display: flex;
    align-items: center;
    gap: var(--space-8);
    padding-inline: var(--space-16);
  }

  header {
    min-height: 22px;
  }

  /* Label/Strong: the group label is quieter than the rows it names. */
  h2 {
    flex: 1;
    font-family: var(--font-body);
    font-size: var(--size-label);
    line-height: var(--leading-label);
    font-weight: 700;
    color: var(--text-secondary);
  }

  footer {
    gap: var(--space-12);
  }

  p {
    flex: 1;
    color: var(--text-secondary);
    font-size: var(--size-label);
    line-height: var(--leading-label);
  }

  .card {
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    background: var(--bg-surface);
    overflow: hidden;
  }

  .sunken {
    background: var(--bg-surface-secondary);
  }
</style>
