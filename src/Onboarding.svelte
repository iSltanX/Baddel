<script lang="ts">
  /**
   * The welcome window — `A2 — Welcome / First Run` in 06 — Onboarding. Four steps: what
   * Baddel is, how a conversion goes, the one permission, and a first real conversion in a
   * field of its own. Success is a new user reaching that first conversion from here alone.
   */
  import Button from './lib/components/Button.svelte'
  import Callout from './lib/components/Callout.svelte'
  import HowStep from './lib/components/HowStep.svelte'
  import Icon from './lib/components/Icon.svelte'
  import Keycap from './lib/components/Keycap.svelte'
  import MenuBarLegend from './lib/components/MenuBarLegend.svelte'
  import PermissionCard from './lib/components/PermissionCard.svelte'
  import PracticeField from './lib/components/PracticeField.svelte'
  import WelcomeProgress from './lib/components/WelcomeProgress.svelte'
  import { toGlyphs } from './lib/accelerator'
  import {
    app,
    appNeedsMove,
    convertText,
    direction,
    finishOnboarding,
    onConversion,
    requestPermission,
    reveal,
    revealAppInFinder,
    t,
    type Outcome,
    type Settings,
  } from './lib/state.svelte'
  import iconUrl from './assets/app-icon.png'

  const STEPS = 4
  /** How long the "granted" confirmation stays before the window moves on by itself. */
  const ADVANCE_AFTER_MS = 1200
  /** Outcomes that mean the shortcut reached Baddel but the word did not change. */
  const NOTHING: Outcome[] = ['no-permission', 'no-text', 'unchanged', 'failed', 'paused', 'no-layouts']

  const settings = $derived(app.settings as Settings)
  const keys = $derived(toGlyphs(settings.shortcutConvert))

  const requested = Number(new URLSearchParams(location.search).get('step'))
  let step = $state(requested >= 1 && requested <= STEPS ? requested : 1)

  /** Running from the disk image or a translocated download: ask for a move first. */
  let needsMove = $state(false)
  $effect(() => {
    void appNeedsMove().then((value) => (needsMove = value))
  })

  // ── Step 3: the permission ─────────────────────────────────────────────────
  /** System Settings is open and Baddel is watching for the switch. */
  let checking = $state(false)
  /** Granted while this step was on screen: confirm it, then move on by itself. */
  let advancing = $state(false)
  let grantedBefore = app.permission

  $effect(() => {
    const granted = app.permission
    if (granted && !grantedBefore && step === 3) {
      advancing = true
      setTimeout(() => {
        advancing = false
        if (step === 3) step = 4
      }, ADVANCE_AFTER_MS)
    }
    if (!granted) advancing = false
    grantedBefore = granted
  })

  // ── Step 4: the first conversion ───────────────────────────────────────────
  let practice = $state(t('onboarding.practice.word'))
  let practiceInput = $state<HTMLInputElement>()
  let outcome = $state<'ready' | 'success' | 'nothing' | 'secure'>('ready')

  /**
   * The step teaches by doing: the user puts the caret after the word and presses the real
   * shortcut, which converts it in place like in any other app. The button (and Return) runs
   * the same conversion for anyone whose shortcut does not reach the field.
   */
  async function tryIt() {
    const result = await convertText(practice)
    if (result && result !== practice) {
      practice = result
      outcome = 'success'
    } else {
      outcome = 'nothing'
    }
  }

  // A conversion anywhere while this step is open was this field: the window has focus.
  // The event carries the outcome only; the converted word is read back from the field.
  $effect(() => {
    if (step !== 4) return
    const stop = onConversion((result) => {
      if (outcome === 'success') return
      if (result === 'converted' || result === 'extended') {
        practice = practiceInput?.value ?? practice
        outcome = 'success'
      } else if (result === 'blocked') {
        outcome = 'secure'
      } else if (NOTHING.includes(result)) {
        outcome = 'nothing'
      }
    })
    return () => void stop.then((unlisten) => unlisten())
  })

  // The field takes the focus on arrival, with the caret after the word.
  $effect(() => {
    if (step !== 4 || !practiceInput) return
    practiceInput.focus()
    practiceInput.setSelectionRange(practice.length, practice.length)
  })

  // ── Footer ─────────────────────────────────────────────────────────────────
  const primaryLabel = $derived(
    step === 1 ? t('onboarding.start') : step === STEPS ? t('onboarding.done') : t('onboarding.continue'),
  )
  const primaryDisabled = $derived((step === 3 && !app.permission) || (step === 4 && outcome !== 'success'))

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
       and what the window is dragged by. The traffic lights float over it. -->
  <header data-tauri-drag-region></header>

  <main class="step-{step}" class:compact={step === 1 && needsMove}>
    {#if step === 1}
      <img class="app-icon" src={iconUrl} alt="" width={needsMove ? 72 : 88} height={needsMove ? 72 : 88} />
      <h1>{t('onboarding.welcome.title')}</h1>
      <p class="body">{t('onboarding.welcome.body')}</p>
      <div class="example">
        <span class="pair">
          <bdi class="from">{t('onboarding.welcome.sampleFrom')}</bdi>
          <span class="arrow" aria-hidden="true">{direction() === 'rtl' ? '←' : '→'}</span>
          <bdi class="to">{t('onboarding.welcome.sampleTo')}</bdi>
        </span>
        <span class="shortcut">
          <span class="label">{t('onboarding.welcome.shortcutLabel')}</span>
          <span class="keys ltr">
            {#each keys as key (key)}<Keycap label={key} size="m" />{/each}
          </span>
        </span>
      </div>
      {#if needsMove}
        <div class="stretch">
          <Callout tone="warning" title={t('onboarding.welcome.moveTitle')} body={t('onboarding.welcome.moveBody')}>
            {#snippet action()}
              <Button onclick={revealAppInFinder}>{t('onboarding.welcome.showInFinder')}</Button>
            {/snippet}
          </Callout>
        </div>
      {/if}
    {:else if step === 2}
      <span class="hero"><Icon name="swap" size={28} /></span>
      <h1>{t('onboarding.how.title')}</h1>
      <p class="body">{t('onboarding.how.body')}</p>
      <div class="how">
        <HowStep stage="type" number={1} label={t('onboarding.how.type')} from={t('onboarding.welcome.sampleFrom')} to={t('onboarding.welcome.sampleTo')} {keys} />
        <span class="next"><Icon name="chevronRight" size={14} flip /></span>
        <HowStep stage="press" number={2} label={t('onboarding.how.press')} from={t('onboarding.welcome.sampleFrom')} to={t('onboarding.welcome.sampleTo')} {keys} />
        <span class="next"><Icon name="chevronRight" size={14} flip /></span>
        <HowStep stage="fixed" number={3} label={t('onboarding.how.fixed')} from={t('onboarding.welcome.sampleFrom')} to={t('onboarding.welcome.sampleTo')} {keys} />
      </div>
      <p class="note">{t('onboarding.how.note')}</p>
      <MenuBarLegend />
    {:else if step === 3}
      <span class="hero"><Icon name="accessibility" size={28} /></span>
      <h1>{t('onboarding.permission.title')}</h1>
      <p class="body">{t('onboarding.permission.body')}</p>
      {#if checking && !app.permission}
        <!-- Where to look in System Settings, drawn simply: the row and its switch. -->
        <div class="system stretch">
          <p>{t('onboarding.permission.path')}</p>
          <div class="system-row">
            <img src={iconUrl} alt="" width="22" height="22" />
            <strong>{t('onboarding.permission.appName')}</strong>
            <span class="system-switch" aria-hidden="true"><span></span></span>
          </div>
        </div>
      {:else}
        <ul class="reassurance stretch">
          <li><Icon name="shield" />{t('onboarding.permission.local')}</li>
          <li><Icon name="noWifi" />{t('onboarding.permission.noServer')}</li>
          <li><Icon name="noSave" />{t('onboarding.permission.noSave')}</li>
        </ul>
      {/if}
      <div class="stretch"><PermissionCard {checking} onrequest={() => (checking = true)} /></div>
      {#if advancing}
        <p class="advancing" role="status">
          {t('onboarding.permission.advancing')}
          <span class="spin"><Icon name="spinner" size={16} /></span>
        </p>
      {/if}
    {:else}
      <span class="hero"><Icon name="keyboard" size={28} /></span>
      <h1>{t('onboarding.practice.title')}</h1>
      <p class="body">{t('onboarding.practice.body')}</p>
      <PracticeField
        bind:value={practice}
        bind:input={practiceInput}
        done={outcome === 'success'}
        label={t('onboarding.practice.title')}
        onenter={tryIt}
      />
      {#if outcome === 'success'}
        <div class="stretch">
          <Callout tone="success" title={t('onboarding.practice.successTitle')} body={t('onboarding.practice.successBody')} />
        </div>
      {:else}
        <div class="press stretch">
          <span class="label">{t('onboarding.practice.press')}</span>
          <span class="keys ltr">
            {#each keys as key (key)}<Keycap label={key} size="m" />{/each}
          </span>
          <span class="spacer"></span>
          <Button onclick={tryIt}>{t('onboarding.practice.tryButton')}</Button>
        </div>
        {#if outcome === 'nothing'}
          <div class="stretch">
            <Callout tone="warning" title={t('onboarding.practice.nothingTitle')} body={t('onboarding.practice.nothingBody')}>
              {#snippet action()}
                <Button onclick={requestPermission}>{t('permission.openSystemSettings')}</Button>
              {/snippet}
            </Callout>
          </div>
        {:else if outcome === 'secure'}
          <div class="stretch">
            <Callout tone="info" title={t('onboarding.practice.secureTitle')} body={t('onboarding.practice.secureBody')} />
          </div>
        {/if}
      {/if}
    {/if}
  </main>

  <footer>
    <span class="skip">
      {#if !(step === STEPS && outcome === 'success')}
        <Button variant="plain" onclick={() => finishOnboarding()}>{t('onboarding.skip')}</Button>
      {/if}
    </span>
    <span class="progress"><WelcomeProgress {step} steps={STEPS} /></span>
    <div class="actions">
      {#if step > 1}
        <Button size="large" onclick={() => (step -= 1)}>{t('onboarding.back')}</Button>
      {/if}
      <Button variant="primary" size="large" disabled={primaryDisabled} onclick={primary}>{primaryLabel}</Button>
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
    gap: var(--space-16);
    min-height: 0;
    padding: var(--space-32) var(--space-48) var(--space-24);
    text-align: center;
    /* The three moments of step 2 run into the side margins on purpose; only the window
       edge clips. */
    overflow-x: hidden;
    overflow-y: auto;
  }

  /* The steps that carry more — how it works with its menu bar legend, the permission, and
     the "move it first" note — sit tighter, so the window keeps its height. */
  main.step-2,
  main.step-3,
  main.compact {
    gap: var(--space-12);
    padding-top: var(--space-24);
  }

  main.compact {
    padding-top: var(--space-20);
  }

  .app-icon {
    display: block;
    flex: none;
  }

  .hero {
    display: grid;
    place-items: center;
    width: 64px;
    height: 64px;
    flex: none;
    border-radius: var(--radius-lg);
    background: var(--bg-selected);
    color: var(--text-brand);
  }

  /* Display/Brand. */
  h1 {
    font-size: var(--size-display);
    line-height: var(--leading-display);
    white-space: pre-line;
  }

  .body {
    max-width: 462px;
    color: var(--text-secondary);
  }

  .stretch {
    align-self: stretch;
  }

  /* Step 1: the conversion itself, as big as the window allows. */
  .example {
    display: flex;
    flex-direction: column;
    align-items: center;
    align-self: stretch;
    gap: var(--space-12);
    padding: var(--space-16) 0;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    background: var(--bg-surface);
  }

  .pair {
    display: flex;
    align-items: center;
    gap: var(--space-16);
  }

  .from {
    color: var(--text-tertiary);
    font-size: 26px;
    line-height: 30px;
  }

  /* The coral dot of this screen: the one mark that says "this is the moment". */
  .arrow {
    color: var(--accent);
    font-family: var(--font-latin);
    font-size: 22px;
    line-height: 26px;
  }

  .to {
    color: var(--text-brand);
    font-family: var(--font-latin);
    font-size: 30px;
    font-weight: 600;
    line-height: 36px;
  }

  .shortcut,
  .press {
    display: flex;
    align-items: center;
    gap: var(--space-8);
  }

  .label {
    color: var(--text-secondary);
    font-size: var(--size-label);
    line-height: var(--leading-label);
  }

  .keys {
    display: inline-flex;
    gap: var(--space-4);
  }

  .spacer {
    flex: 1;
  }

  /* Step 2: three moments, wider than the text column above them. */
  .how {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    flex: none;
  }

  .next {
    display: grid;
    color: var(--text-tertiary);
  }

  .note {
    color: var(--text-tertiary);
    font-size: var(--size-label);
    line-height: var(--leading-label);
  }

  /* Step 3. */
  .reassurance {
    margin: 0;
    padding: 0;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    background: var(--bg-surface);
    list-style: none;
    overflow: hidden;
    text-align: start;
  }

  .reassurance li {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-12);
    min-height: 40px;
    padding: 0 var(--space-16);
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

  .system {
    display: flex;
    flex-direction: column;
    gap: var(--space-8);
    padding: var(--space-12) var(--space-16);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    background: var(--bg-surface-secondary);
    text-align: start;
  }

  .system p {
    color: var(--text-secondary);
    font-size: var(--size-label);
    line-height: var(--leading-label);
  }

  .system-row {
    display: flex;
    align-items: center;
    gap: var(--space-8);
    padding: var(--space-8) var(--space-12);
    border-radius: var(--radius-md);
    background: var(--bg-surface);
  }

  .system-row strong {
    flex: 1;
  }

  .system-switch {
    display: flex;
    justify-content: flex-end;
    width: 32px;
    height: 18px;
    padding: var(--space-2);
    border-radius: var(--radius-full);
    background: var(--action-primary);
    direction: ltr;
  }

  .system-switch span {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--control-knob);
  }

  .advancing {
    display: flex;
    align-items: center;
    gap: var(--space-8);
    color: var(--text-brand);
    font-size: var(--size-label);
    line-height: var(--leading-label);
    font-weight: 700;
  }

  .spin {
    display: grid;
    animation: spin 900ms linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  footer {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-8);
    height: 68px;
    flex: none;
    padding: 0 var(--space-24);
    border-top: 1px solid var(--border-subtle);
  }

  .skip {
    min-width: 56px;
  }

  .actions {
    display: flex;
    gap: var(--space-8);
  }

  /* Centred on the window, not between the buttons, so it holds still as they change. */
  .progress {
    position: absolute;
    inset-inline: 0;
    display: flex;
    justify-content: center;
    pointer-events: none;
  }
</style>
