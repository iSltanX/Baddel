<script lang="ts">
  import { ICON_PATHS, type IconName } from '../icons'

  interface Props {
    name: IconName
    size?: number
    /** Pointers follow the reading direction; key glyphs and logos never do. */
    flip?: boolean
  }

  const { name, size = 18, flip = false }: Props = $props()
  /** From 18px up the line stays 1.5px on screen; smaller marks scale it with the glyph. */
  const stroke = $derived(size >= 18 ? (1.5 * 24) / size : 1.5)
</script>

<svg
  class:flip
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  <path d={ICON_PATHS[name]} />
</svg>

<style>
  svg {
    display: block;
    flex: none;
  }

  :global([dir='rtl']) .flip {
    transform: scaleX(-1);
  }
</style>
