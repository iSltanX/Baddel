import { mount } from 'svelte'

import App from './App.svelte'
import './lib/theme.css'
import { start } from './lib/state.svelte'

// Nothing renders before the settings are in hand: a window that flashes the
// defaults and then corrects itself looks broken.
start().then(() => {
  mount(App, { target: document.getElementById('app')! })
})
