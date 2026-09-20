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
    requestPermission,
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

  const primaryLabel = $derived(
    step === 1
      ? t('onboarding.next')
      : step === 2
        ? app.permission
          ? t('onboarding.next')
          : t('onboarding.openSystemSettings')
        : t('onboarding.start'),
  )

  function primary() {
    if (step === 3) {
      void finishOnboarding()
      return
    }
    if (step === 2 && !app.permission) {
      void requestPermission()
      return
    }
    step += 1
  }

  $effect(() => {
    void reveal()
  })
</script>

<div class="window">
  <header></header>

  {#if step > 1}
    <button class="back" type="button" aria-label={t('common.cancel')} onclick={() => (step -= 1)}>
      <!-- Flipped with the reading direction, unlike the glyphs on the keycaps. -->
      <Icon name="chevron" size={18} flip />
    </button>
  {/if}

  <main>
    {#if step === 1}
      <img src={iconUrl} alt="" width="72" height="72" />
      <h1>{t('onboarding.step1.title')}</h1>
      <p class="body">{t('onboarding.step1.body')}</p>
      <div class="card samples">
        <ConversionSample
          from={t('onboarding.step1.sample1From')}
          to={t('onboarding.step1.sample1To')}
          size="hero"
        />
        <hr />
        <ConversionSample
          from={t('onboarding.step1.sample2From')}
          to={t('onboarding.step1.sample2To')}
          size="hero"
        />
      </div>
    {:else if step === 2}
      <h1>{t('onboarding.step2.title')}</h1>
      <p class="body">{t('onboarding.step2.body')}</p>
      <ul class="reassurance">
        <li><Icon name="noEye" size={18} />{t('onboarding.step2.reassurance.noKeylogging')}</li>
        <li><Icon name="noWifi" size={18} />{t('onboarding.step2.reassurance.noInternet')}</li>
        <li><Icon name="noText" size={18} />{t('onboarding.step2.reassurance.noTextStorage')}</li>
      </ul>
      <div class="stretch"><PermissionCard /></div>
    {:else}
      <h1>{t('onboarding.step3.title')}</h1>
      <div class="recorder">
        <ShortcutRecorder
          value={settings.shortcutConvert}
          {conflict}
          label={t('settings.shortcuts.rows.convert')}
          onrecord={async (accelerator) => (conflict = await setShortcut('convert', accelerator))}
        />
        <span class="caption">{t('onboarding.step3.recorderCaption')}</span>
      </div>
      <div class="card practice">
        <input
          class="field"
          bind:value={practice}
          aria-label={t('onboarding.step3.title')}
          onkeydown={(event) => event.key === 'Enter' && tryIt()}
        />
      </div>
      {#if converted}
        <p class="success" role="status">
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
    {/if}
  </main>

  <footer>
    <Button variant="plain" onclick={() => finishOnboarding()}>{t('onboarding.skip')}</Button>
    <div class="dots" aria-hidden="true">
      {#each { length: STEPS } as _, index (index)}
        <span class="dot" class:active={index + 1 === step}></span>
      {/each}
    </div>
    <Button variant="primary" size="large" onclick={primary}>{primaryLabel}</Button>
  </footer>
</div>

<style>
  .window {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  /* An empty strip under the transparent title bar. The traffic lights live there,
     and macOS puts them on whichever side its own language reads from. */
  header {
    height: 38px;
    flex: none;
  }

  /* Below that strip, so it cannot collide with them on either side. */
  .back {
    position: absolute;
    top: 42px;
    inset-inline-start: 14px;
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border: none;
    border-radius: var(--radius-control);
    background: none;
    color: var(--text-secondary);
  }

  .back:hover {
    background: var(--overlay-hover);
  }

  main {
    display: flex;
    flex: 1;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 14px 44px;
    text-align: center;
    overflow-y: auto;
  }

  /* Cards fill the width; headings and the icon stay centred on their own. */
  .stretch {
    width: 100%;
  }

  img {
    border-radius: 16px;
  }

  h1 {
    font-size: var(--size-large);
    line-height: var(--leading-large);
    font-weight: 700;
  }

  .body {
    margin: 0;
    max-width: 42ch;
    color: var(--text-secondary);
  }

  .card {
    width: 100%;
    padding: 14px 16px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-card);
    background: var(--bg-surface);
  }

  .samples {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
  }

  hr {
    width: 100%;
    margin: 0;
    border: none;
    border-top: 1px solid var(--border-subtle);
  }

  .reassurance {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 100%;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .reassurance li {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--text-secondary);
    text-align: start;
  }

  .recorder {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
  }

  .caption {
    color: var(--text-secondary);
    font-size: var(--size-small);
  }

  .practice {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .field {
    flex: 1;
    min-width: 0;
    height: 32px;
    padding-inline: 10px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-control);
    background: var(--bg-sunken);
    font-size: var(--size-body);
    user-select: text;
  }

  .helper,
  .success {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--size-small);
  }

  .success {
    color: var(--state-success);
  }

  .keys {
    display: inline-flex;
    gap: 3px;
    direction: ltr;
  }

  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex: none;
    padding: 14px 20px;
    border-top: 1px solid var(--border-subtle);
  }

  .dots {
    display: flex;
    gap: 5px;
  }

  .dot {
    width: 10px;
    height: 3px;
    border-radius: 2px;
    background: var(--border-subtle);
    transition: width 150ms ease-out;
  }

  .active {
    width: 28px;
    background: var(--accent-primary);
  }
</style>
