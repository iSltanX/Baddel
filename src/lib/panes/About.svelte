<script lang="ts">
  import Button from '../components/Button.svelte'
  import DiagnosticsDetails from '../components/DiagnosticsDetails.svelte'
  import FormRow from '../components/FormRow.svelte'
  import GroupCard from '../components/GroupCard.svelte'
  import Icon from '../components/Icon.svelte'
  import { appVersion, closeWindow, copyDiagnostics, openExternal, openOnboarding, openReport, t } from '../state.svelte'
  import iconUrl from '../../assets/app-icon.png'

  const REPOSITORY = 'https://github.com/iSltanX/Baddel'
  const REPOSITORY_LABEL = 'github.com/iSltanX/Baddel'

  let version = $state('')
  $effect(() => {
    void appVersion().then((value) => (version = value))
  })

  let copied = $state(false)
  /** "What's included?" — what the copy holds and what it never holds. */
  let included = $state(false)

  async function copy() {
    await copyDiagnostics()
    copied = true
    setTimeout(() => (copied = false), 1600)
  }

  /** The maker's other apps; their tints live in theme.css until their own icons ship. */
  const makers = ['raff', 'luma', 'nafidh'] as const

  async function welcome() {
    await openOnboarding()
    await closeWindow()
  }
</script>

<div class="identity">
  <img src={iconUrl} alt="" width="104" height="104" />
  <h1>{t('settings.about.appName')}</h1>
  <p class="version">{t('settings.about.version', { version })}</p>
  <p class="tagline">{t('settings.about.tagline')}</p>
</div>

<GroupCard>
  <button class="link" type="button" onclick={() => openExternal(REPOSITORY)}>
    <FormRow first title={t('settings.about.sourceOnGithub')}>
      <span class="affordance"><Icon name="external" size={16} /></span>
    </FormRow>
  </button>
  <button class="link" type="button" onclick={openReport}>
    <FormRow title={t('settings.about.reportProblem')}>
      <span class="affordance"><Icon name="chevronRight" size={16} flip /></span>
    </FormRow>
  </button>
  <FormRow title={t('settings.about.copyDiagnostics')} description={t('settings.about.copyDiagnosticsDescription')}>
    {#snippet detail()}
      <button
        class="disclosure"
        type="button"
        aria-expanded={included}
        aria-controls="diagnostics-details"
        onclick={() => (included = !included)}
      >
        {t('settings.about.whatsIncluded')}
        <span aria-hidden="true">{included ? '▴' : '▾'}</span>
      </button>
    {/snippet}
    <Button onclick={copy}>{copied ? t('settings.about.copied') : t('settings.about.copy')}</Button>
  </FormRow>
  {#if included}
    <div class="details" id="diagnostics-details"><DiagnosticsDetails /></div>
  {/if}
  <button class="link" type="button" onclick={() => openExternal(`${REPOSITORY}/blob/main/LICENSE`)}>
    <FormRow title={t('settings.about.license')}>
      <span class="value">MIT</span>
      <span class="affordance"><Icon name="external" size={16} /></span>
    </FormRow>
  </button>
  <button class="link" type="button" onclick={welcome}>
    <FormRow title={t('settings.about.showWelcome')}>
      <!-- Forward: right in English, mirrored to the left in Arabic. -->
      <span class="affordance"><Icon name="chevronRight" size={16} flip /></span>
    </FormRow>
  </button>
</GroupCard>

<GroupCard title={t('settings.about.fromSameMaker')}>
  <ul class="makers">
    {#each makers as maker (maker)}
      {@const name = t(`settings.about.makerApps.${maker}`)}
      <li>
        <span class="maker-icon {maker}" aria-hidden="true">{name.charAt(0)}</span>
        {name}
      </li>
    {/each}
  </ul>
</GroupCard>

<footer>
  <p class="made-by">
    <span>{t('settings.about.madeByLabel')}</span>
    <strong>{t('settings.about.madeByName')}</strong>
  </p>
  <button class="repo" type="button" onclick={() => openExternal(REPOSITORY)}>
    <bdi>{REPOSITORY_LABEL}</bdi>
    <Icon name="external" size={12} />
  </button>
  <p class="promise">{t('settings.about.promise')}</p>
</footer>

<style>
  .identity {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-4);
    padding-bottom: var(--space-4);
    text-align: center;
  }

  img {
    display: block;
  }

  h1 {
    font-size: var(--size-title);
    line-height: var(--leading-title);
  }

  .version,
  .tagline {
    color: var(--text-secondary);
  }

  .version {
    font-size: var(--size-label);
    line-height: var(--leading-label);
  }

  /* A link is a whole row: the entire width is the target, not just the words. */
  .link {
    display: block;
    width: 100%;
    padding: 0;
    border: none;
    background: none;
    text-align: start;
    transition: background-color 120ms ease-out;
  }

  .link:hover {
    background: var(--overlay-hover);
  }

  .link:active {
    background: var(--overlay-pressed);
  }

  .link:focus-visible {
    box-shadow: inset var(--focus-ring);
  }

  .value {
    color: var(--text-secondary);
    font-family: var(--font-latin);
    font-size: var(--size-latin);
  }

  .affordance {
    display: grid;
    color: var(--text-tertiary);
  }

  .disclosure {
    align-self: flex-start;
    display: inline-flex;
    gap: var(--space-4);
    padding: 0;
    border: none;
    border-radius: var(--radius-xs);
    background: none;
    color: var(--text-brand);
    font-size: var(--size-label);
    line-height: var(--leading-label);
    font-weight: 700;
  }

  .disclosure:hover {
    text-decoration: underline;
  }

  .details {
    padding: 0 var(--space-16) var(--space-12);
  }

  .makers {
    display: flex;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .makers li {
    display: flex;
    flex: 1;
    align-items: center;
    justify-content: center;
    gap: var(--space-12);
    padding: var(--space-12) var(--space-16);
    border-inline-start: 1px solid var(--border-subtle);
  }

  .makers li:first-child {
    border-inline-start: none;
  }

  .maker-icon {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    flex: none;
    border-radius: var(--radius-md);
    color: var(--tint-on);
    font-family: var(--font-heading);
    font-size: var(--size-section);
    font-weight: 700;
    line-height: 1;
  }

  .raff {
    background: var(--tint-raff);
  }

  .luma {
    background: var(--tint-luma);
  }

  .nafidh {
    background: var(--tint-nafidh);
  }

  footer {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-8);
    text-align: center;
  }

  .made-by {
    display: flex;
    gap: var(--space-8);
    font-size: var(--size-label);
    line-height: var(--leading-label);
    color: var(--text-secondary);
  }

  .made-by strong {
    color: var(--text-primary);
  }

  .repo {
    display: inline-flex;
    align-items: center;
    gap: var(--space-4);
    padding: 0 var(--space-4);
    border: none;
    border-radius: var(--radius-xs);
    background: none;
    color: var(--text-brand);
    font-family: var(--font-latin);
    font-size: var(--size-latin-small);
    line-height: var(--leading-label);
  }

  .repo:hover {
    text-decoration: underline;
  }

  .promise {
    color: var(--text-tertiary);
    font-size: var(--size-caption);
    line-height: var(--leading-caption);
  }
</style>
