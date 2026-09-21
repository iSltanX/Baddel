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
    /* The 2px lower edge is inside the box, so the label sits 1px high to stay centred on the face. */
    padding: 0 4px 2px;
    border: 1px solid var(--keycap-edge);
    border-bottom-width: 2px;
    border-radius: var(--radius-keycap);
    background: var(--keycap-top);
    color: var(--text-primary);
    font-family: var(--font-latin);
    font-size: 10px;
    font-weight: 500;
    line-height: 12px;
    direction: ltr;
    unicode-bidi: isolate;
    box-sizing: border-box;
  }

  .m {
    min-width: 24px;
    height: 24px;
    padding: 0 6px 2px;
    border-radius: var(--radius-control);
    font-size: 12px;
    line-height: 16px;
  }

  .pressed {
    padding-top: 2px;
    padding-bottom: 0;
    border-bottom-width: 1px;
    background: var(--bg-sunken);
  }
</style>
