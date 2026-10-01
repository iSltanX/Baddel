<script lang="ts">
  /**
   * `Welcome / Progress` in 02 — Components: one bar per step. The current step is the long
   * mint bar, the steps behind it are brand marks, the ones ahead are quiet.
   */
  interface Props {
    step: number
    steps: number
  }

  const { step, steps }: Props = $props()
</script>

<div class="progress" role="progressbar" aria-valuemin={1} aria-valuemax={steps} aria-valuenow={step}>
  {#each { length: steps } as _, index (index)}
    <span class:done={index + 1 < step} class:current={index + 1 === step}></span>
  {/each}
</div>

<style>
  .progress {
    display: flex;
    align-items: center;
    gap: var(--space-8);
  }

  span {
    width: 8px;
    height: 4px;
    border-radius: var(--radius-full);
    background: var(--border-default);
    transition: width 200ms ease-out;
  }

  .done {
    background: var(--text-brand);
  }

  .current {
    width: 24px;
    background: var(--action-primary);
  }
</style>
