<script lang="ts">
  import Button from './lib/components/Button.svelte'
  import ConversionSample from './lib/components/ConversionSample.svelte'
  import Icon from './lib/components/Icon.svelte'
  import Keycap from './lib/components/Keycap.svelte'
  import PermissionCard from './lib/components/PermissionCard.svelte'
  import ShortcutRecorder from './lib/components/ShortcutRecorder.svelte'
  import { listen } from '@tauri-apps/api/event'
  import { toGlyphs } from './lib/accelerator'
  import {
    app,
    convertText,
    finishOnboarding,
    reveal,
    setShortcut,
    t,
    type Settings,
  } from './lib/state.svelte'
  import iconUrl from './assets/app-icon.png'

  const STEPS = 3

  const settings = $derived(app.settings as Settings)

  let step = $state(Number(new URLSearchParams(location.search).get('step')) || 1)
  let practice = $state(t('onboarding.step3.practiceText'))
  let converted = $state(false)
  let conflict = $state(false)

  /**
   * The step teaches by doing: the user puts the caret in the field and presses the
   * real shortcut, which converts it in place like any other app. Return does the
   * same thing for anyone whose shortcut is not bound yet.
   */
  async function tryIt() {
    const result = await convertText(practice)
    if (result && result !== practice) {
      practice = result
      converted = true
    }
  }

  // A conversion anywhere while this step is open was this field: the window has focus.
  $effect(() => {
    // Only the app itself reports conversions; in a browser there are none to hear.
    if (step !== 3 || !('__TAURI_INTERNALS__' in window)) return
    const stop = listen<string>('conversion', (event) => {
      if (event.payload === 'converted' || event.payload === 'extended') converted = true
    })
    return () => void stop.then((unlisten) => unlisten())
  })

  const primaryLabel = $derived(step === STEPS ? t('onboarding.start') : t('onboarding.next'))
  /**
   * One primary action per screen. While the permission is missing, that action is
   * the card's own button, so "Next" steps back to a secondary one.
   */
  const primaryVariant = $derived(step === 2 && !app.permission ? 'secondary' : 'primary')

  function primary() {
    if (step === STEPS) {
      void finishOnboarding()
      return
    }
    step += 1
  }

  $effect(() => {
    void reveal()
  })
</script>

<div class="window">
  <!-- The window's title bar is an overlay: this empty strip is what shows through it,
       and what the window is dragged by. The traffic lights float over it, on
       whichever side macOS reads from. -->
  <header data-tauri-drag-region></header>

  <main>
    <div class="hero">
      {#if step === 1}
        <img src={iconUrl} alt="" width="104" height="104" />
      {:else}
        <span class="glyph"><Icon name={step === 2 ? 'accessibility' : 'keyboard'} size={32} /></span>
      {/if}
    </div>

    <div class="heading">
      <h1>{t(`onboarding.step${step}.title`)}</h1>
      <p>{t(`onboarding.step${step}.body`)}</p>
    </div>

    {#if step === 1}
      <div class="card">
        <div class="shortcut-row">
          <span>{t('onboarding.step1.shortcutLabel')}</span>
          <span class="keys">
            {#each toGlyphs(settings.shortcutConvert) as glyph (glyph)}
              <Keycap label={glyph} size="m" />
            {/each}
          </span>
        </div>
        <div class="samples">
          <ConversionSample from={t('onboarding.step1.sample1From')} to={t('onboarding.step1.sample1To')} size="hero" />
          <ConversionSample from={t('onboarding.step1.sample2From')} to={t('onboarding.step1.sample2To')} size="hero" />
        </div>
      </div>
    {:else if step === 2}
      <ul class="card reassurance">
        <li><Icon name="noEye" />{t('onboarding.step2.reassurance.noKeylogging')}</li>
        <li><Icon name="noWifi" />{t('onboarding.step2.reassurance.noInternet')}</li>
        <li><Icon name="noSave" />{t('onboarding.step2.reassurance.noTextStorage')}</li>
      </ul>
      <div class="stretch"><PermissionCard /></div>
    {:else}
      <div class="practice" class:converted>
        <input
          bind:value={practice}
          dir="auto"
          aria-label={t('onboarding.step3.title')}
          onkeydown={(event) => event.key === 'Enter' && tryIt()}
        />
        {#if converted}<span class="done"><Icon name="checkCircle" size={22} /></span>{/if}
      </div>
      {#if converted}
        <p class="helper success" role="status">
          <Icon name="checkCircle" size={16} />
          {t('onboarding.step3.successMessage')}
        </p>
      {:else}
        <p class="helper">
          {t('onboarding.step3.practiceHelper')}
          <span class="keys">
            {#each toGlyphs(settings.shortcutConvert) as glyph (glyph)}
              <Keycap label={glyph} />
            {/each}
          </span>
        </p>
      {/if}
      <div class="card your-shortcut">
        <div class="text">
          <span>{t('onboarding.step3.recorderLabel')}</span>
          <span class="caption" class:warning={conflict}>
            {conflict ? t('shortcut.recorder.conflict') : t('onboarding.step3.recorderCaption')}
          </span>
        </div>
        <ShortcutRecorder
          value={settings.shortcutConvert}
          {conflict}
          label={t('settings.shortcuts.rows.convert')}
          onrecord={async (accelerator) => (conflict = await setShortcut('convert', accelerator))}
        />
      </div>
    {/if}
  </main>

  <footer>
    <Button variant="plain" onclick={() => finishOnboarding()}>{t('onboarding.skip')}</Button>
    <div class="dots" aria-hidden="true">
      {#each { length: STEPS } as _, index (index)}
        <span class="dot" class:active={index + 1 === step}></span>
      {/each}
    </div>
    <div class="actions">
      {#if step > 1}
        <Button size="large" onclick={() => (step -= 1)}>{t('onboarding.back')}</Button>
      {/if}
      <Button variant={primaryVariant} size="large" onclick={primary}>{primaryLabel}</Button>
    </div>
  </footer>
</div>

<style>
  .window {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  header {
    height: 28px;
    flex: none;
  }

  /* Anchored to the top, so the title sits on the same line in every step. */
  main {
    display: flex;
    flex: 1;
    flex-direction: column;
    align-items: center;
    padding: 8px 48px 24px;
    text-align: center;
    overflow-y: auto;
  }

  /* One fixed slot for the step's picture keeps the heading from jumping between steps. */
  .hero {
    display: grid;
    place-items: center;
    height: 96px;
    flex: none;
  }

  .glyph {
    display: grid;
    place-items: center;
    width: 72px;
    height: 72px;
    border-radius: 50%;
    background: var(--accent-soft);
    color: var(--accent-primary);
  }

  .heading {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding-block: 16px 24px;
  }

  h1 {
    font-size: var(--size-large);
    line-height: var(--leading-large);
    font-weight: 700;
  }

  .heading p {
    margin: 0;
    max-width: 464px;
    color: var(--text-secondary);
  }

  /* Cards fill the width; headings and the picture stay centred on their own. */
  .card,
  .stretch,
  .practice {
    width: 100%;
  }

  .card {
    margin: 0;
    padding: 0;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-card);
    background: var(--bg-surface);
    list-style: none;
    overflow: hidden;
    text-align: start;
  }

  .shortcut-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    min-height: 48px;
    padding: 0 16px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .samples {
    display: flex;
  }

  .samples > :global(*) {
    flex: 1;
    justify-content: center;
    padding: 20px 8px;
  }

  .samples > :global(* + *) {
    border-inline-start: 1px solid var(--border-subtle);
  }

  .reassurance li {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 40px;
    padding: 0 16px;
    color: var(--text-primary);
  }

  .reassurance li :global(svg) {
    color: var(--text-secondary);
  }

  .reassurance li + li::before {
    content: '';
    position: absolute;
    inset-block-start: 0;
    inset-inline: 46px 0;
    height: 1px;
    background: var(--border-subtle);
  }

  .stretch {
    margin-top: 12px;
  }

  /* The practice field is the step: big, focused, and it turns green when it worked. */
  .practice {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 64px;
    padding: 0 20px;
    border: 1.5px solid var(--accent-primary);
    border-radius: var(--radius-card);
    background: var(--bg-surface);
    box-shadow: 0 0 0 var(--focus-width) var(--focus-ring);
  }

  .practice input {
    flex: 1;
    min-width: 0;
    border: none;
    outline: none;
    background: none;
    box-shadow: none;
    font-size: var(--size-sample);
    line-height: var(--leading-sample);
    font-weight: 700;
    caret-color: var(--accent-primary);
    user-select: text;
  }

  .converted {
    border-color: var(--state-success);
    background: var(--state-success-soft);
    box-shadow: none;
  }

  .converted input {
    caret-color: var(--state-success);
  }

  .done {
    display: grid;
    color: var(--state-success);
  }

  .helper {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    margin: 0;
    padding-block: 12px 20px;
    color: var(--text-secondary);
    font-size: var(--size-small);
    line-height: var(--leading-small);
  }

  .success {
    color: var(--state-success);
    font-size: var(--size-body);
    line-height: var(--leading-body);
    font-weight: 700;
  }

  .keys {
    display: inline-flex;
    gap: 4px;
    direction: ltr;
  }

  .helper .keys {
    gap: 3px;
  }

  .your-shortcut {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 56px;
    padding: 10px 16px;
  }

  .your-shortcut .text {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
  }

  .caption {
    color: var(--text-secondary);
    font-size: var(--size-small);
    line-height: var(--leading-small);
  }

  .warning {
    color: var(--state-warning-text);
  }

  footer {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    height: 68px;
    flex: none;
    padding: 0 20px;
    border-top: 1px solid var(--border-subtle);
  }

  .actions {
    display: flex;
    gap: 8px;
  }

  /* Centred on the window, not between the buttons, so it holds still as they change. */
  .dots {
    position: absolute;
    inset-inline: 0;
    display: flex;
    justify-content: center;
    gap: 6px;
    pointer-events: none;
  }

  .dot {
    width: 8px;
    height: 4px;
    border-radius: 999px;
    background: var(--border-strong);
    transition: width 150ms ease-out;
  }

  .active {
    width: 24px;
    background: var(--accent-primary);
  }
</style>
