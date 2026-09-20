<script lang="ts">
  /** One key of a keyboard, as printed on the physical cap. Never mirrored. */
  interface Props {
    label: string
    size?: 's' | 'm'
    pressed?: boolean
  }

  const { label, size = 's', pressed = false }: Props = $props()
</script>

<span class="keycap {size}" class:pressed>{label}</span>

<style>
  .keycap {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 20px;
    height: 20px;
    padding: 0 6px;
    border: 1px solid var(--border-subtle);
    border-bottom-width: 2px;
    border-bottom-color: var(--keycap-edge);
    border-radius: 5px;
    background: linear-gradient(var(--bg-surface), var(--bg-sunken));
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: var(--size-caption);
    line-height: 1;
    direction: ltr;
    unicode-bidi: isolate;
  }

  .m {
    min-width: 26px;
    height: 26px;
    padding: 0 8px;
    font-size: var(--size-small);
  }

  .pressed {
    border-bottom-width: 1px;
    transform: translateY(1px);
    background: var(--bg-sunken);
  }
</style>
