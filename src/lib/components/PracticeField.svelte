<script lang="ts">
  import Icon from './Icon.svelte'

  /**
   * `Welcome / Practice field` in 02 — Components: a real text field holding a word typed
   * in the wrong layout. The user puts the caret after it and presses the real shortcut;
   * Baddel converts it in place, the same way it does in any other app.
   */
  interface Props {
    value: string
    /** Set once the word has been converted; the field then shows the result. */
    done?: boolean
    label: string
    onenter?: () => void
    input?: HTMLInputElement
  }

  let { value = $bindable(), done = false, label, onenter, input = $bindable() }: Props = $props()
</script>

<div class="field" class:done>
  {#if done}
    <!-- The mark leads, on the reading side; the word follows it. -->
    <span class="check"><Icon name="checkCircle" size={20} /></span>
    <bdi class="word result">{value}</bdi>
  {:else}
    <input
      bind:this={input}
      bind:value
      class="word"
      dir="auto"
      aria-label={label}
      spellcheck="false"
      autocomplete="off"
      onkeydown={(event) => event.key === 'Enter' && onenter?.()}
    />
  {/if}
</div>

<style>
  .field {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-12);
    width: 100%;
    height: 72px;
    flex: none;
    border: 1px solid var(--border-default);
    border-radius: var(--radius-lg);
    background: var(--bg-surface);
    transition: border-color 120ms ease-out;
  }

  /* Focused: the ring is the field's own 2px edge in the focus colour. */
  .field:focus-within {
    border: 2px solid var(--border-focus);
  }

  .done {
    border: 2px solid var(--success);
    background: var(--bg-selected);
  }

  .word {
    font-size: 28px;
    font-weight: 700;
    line-height: 32px;
  }

  input {
    width: 100%;
    height: 100%;
    padding: 0 var(--space-16);
    border: none;
    background: none;
    text-align: center;
    caret-color: var(--border-focus);
    user-select: text;
    cursor: text;
  }

  input:focus-visible {
    box-shadow: none;
  }

  .result {
    color: var(--text-brand);
    font-family: var(--font-latin);
    font-weight: 600;
  }

  .check {
    display: grid;
    color: var(--success);
  }
</style>
