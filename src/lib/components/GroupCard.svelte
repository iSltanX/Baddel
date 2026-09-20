<script lang="ts">
  import type { Snippet } from 'svelte'

  /** A grouped form card: an inset surface holding one to six rows. */
  interface Props {
    title?: string
    description?: string
    children: Snippet
  }

  const { title, description, children }: Props = $props()
</script>

<section>
  {#if title}
    <h2>{title}</h2>
  {/if}
  {#if description}
    <p class="description">{description}</p>
  {/if}
  <div class="card">
    {@render children()}
  </div>
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  h2 {
    font-size: var(--size-section);
    line-height: var(--leading-section);
    font-weight: 600;
    padding-inline: 4px;
  }

  .description {
    margin: 0;
    padding-inline: 4px;
    font-size: var(--size-small);
    line-height: var(--leading-small);
    color: var(--text-secondary);
  }

  .card {
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-card);
    background: var(--bg-surface);
    overflow: hidden;
  }
</style>
