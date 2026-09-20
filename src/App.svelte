<script lang="ts">
  /**
   * The root of both windows. Which one this is comes from the URL Rust opened it
   * with, so a single bundle serves them and neither knows about the other.
   */
  import Onboarding from './Onboarding.svelte'
  import Settings from './Settings.svelte'
  import { direction, language } from './lib/state.svelte'

  const which = new URLSearchParams(location.search).get('window')

  // The whole layout mirrors with the interface language.
  $effect(() => {
    document.documentElement.lang = language()
    document.documentElement.dir = direction()
  })
</script>

{#if which === 'onboarding'}
  <Onboarding />
{:else}
  <Settings />
{/if}
