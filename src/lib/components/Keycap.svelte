<script lang="ts">
  /** `Keycap` in 02 — Components: one key, as printed on the physical cap. Never mirrored. */
  interface Props {
    label: string
    size?: 's' | 'm'
    pressed?: boolean
  }

  const { label, size = 's', pressed = false }: Props = $props()
</script>

<span class="keycap {size}" class:pressed>{label}</span>

<style>
  /* SM: 20 high, Keycap/SM. */
  .keycap {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 20px;
    height: 20px;
    /* The 2px lower edge is inside the box, so the label sits 1px high to stay centred on the face. */
    padding: 0 var(--space-4) var(--space-2);
    border: 1px solid var(--keycap-edge);
    border-bottom-width: 2px;
    border-radius: var(--radius-xs);
    background: var(--keycap-top);
    color: var(--text-primary);
    font-family: var(--font-latin);
    font-size: var(--size-keycap-small);
    font-weight: 500;
    line-height: var(--leading-keycap-small);
    direction: ltr;
    unicode-bidi: isolate;
    box-sizing: border-box;
  }

  /* MD: 24 high, Keycap/MD. */
  .m {
    min-width: 24px;
    height: 24px;
    padding: 0 var(--space-8) var(--space-2);
    border-radius: var(--radius-sm);
    font-size: var(--size-keycap);
    line-height: var(--leading-keycap);
  }

  .pressed {
    padding-top: var(--space-2);
    padding-bottom: 0;
    border-bottom-width: 1px;
    background: var(--bg-surface-secondary);
  }
</style>
