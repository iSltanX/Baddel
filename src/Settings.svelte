<script lang="ts">
  import PaneToolbar from './lib/components/PaneToolbar.svelte'
  import About from './lib/panes/About.svelte'
  import Exceptions from './lib/panes/Exceptions.svelte'
  import General from './lib/panes/General.svelte'
  import Layouts from './lib/panes/Layouts.svelte'
  import Shortcuts from './lib/panes/Shortcuts.svelte'
  import { reveal, setContentHeight, setWindowTitle, t } from './lib/state.svelte'

  /** The pane last opened, as macOS settings windows remember it. */
  const REMEMBERED = 'baddel.pane'

  const panes = [
    { id: 'general', component: General, icon: 'general' },
    { id: 'shortcuts', component: Shortcuts, icon: 'keyboard' },
    { id: 'layouts', component: Layouts, icon: 'layouts' },
    { id: 'exceptions', component: Exceptions, icon: 'exceptions' },
    { id: 'about', component: About, icon: 'info' },
  ] as const

  const requested = new URLSearchParams(location.search).get('pane')
  let selected = $state(requested ?? localStorage.getItem(REMEMBERED) ?? 'general')

  const current = $derived(panes.find((pane) => pane.id === selected) ?? panes[0])
  const items = $derived(panes.map((pane) => ({ id: pane.id, label: t(`settings.panes.${pane.id}`), icon: pane.icon })))

  function select(id: string) {
    selected = id
    localStorage.setItem(REMEMBERED, id)
  }

  // The window title is the pane name, and the window is as tall as the pane needs.
  $effect(() => {
    void setWindowTitle(t(`settings.panes.${current.id}`))
  })

  // The window is as tall as the pane needs. Measuring the whole page rather than
  // the pane keeps the toolbar's own height out of the arithmetic.
  $effect(() => {
    const root = document.getElementById('app')
    if (!root) return
    const observer = new ResizeObserver(() => {
      void setContentHeight(Math.ceil(root.scrollHeight))
    })
    observer.observe(root)
    return () => observer.disconnect()
  })

  $effect(() => {
    void reveal()
  })
</script>

<PaneToolbar panes={items} {selected} onselect={select} />

<main>
  <!-- Keying on the pane id tears the old pane down, so no state leaks between them. -->
  {#key current.id}
    <current.component />
  {/key}
</main>

<style>
  main {
    display: flex;
    flex-direction: column;
    gap: var(--group-gap);
    padding: var(--window-pad);
  }
</style>
