<script lang="ts">
  import { ICON_PATHS, type IconName } from '../icons'

  interface Props {
    name: IconName
    size?: number
    /** Pointers follow the reading direction; key glyphs and logos never do. */
    flip?: boolean
  }

  const { name, size = 18, flip = false }: Props = $props()
</script>

<svg
  class:flip
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.5"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  <path d={ICON_PATHS[name]} />
  {#if name === 'general'}
    <circle cx="15.5" cy="8" r="2" />
    <circle cx="9.5" cy="16" r="2" />
  {/if}
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
