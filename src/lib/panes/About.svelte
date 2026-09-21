<script lang="ts">
  import FormRow from '../components/FormRow.svelte'
  import GroupCard from '../components/GroupCard.svelte'
  import Icon from '../components/Icon.svelte'
  import { appVersion, closeWindow, openExternal, openOnboarding, t } from '../state.svelte'
  import iconUrl from '../../assets/app-icon.png'

  const REPOSITORY = 'https://github.com/iSltanX/Baddel'
  const REPOSITORY_LABEL = 'github.com/iSltanX/Baddel'

  let version = $state('')
  $effect(() => {
    void appVersion().then((value) => (version = value))
  })

  const links = [
    { key: 'sourceOnGithub', url: REPOSITORY, value: '' },
    { key: 'reportIssue', url: `${REPOSITORY}/issues`, value: '' },
    { key: 'license', url: `${REPOSITORY}/blob/main/LICENSE`, value: 'MIT' },
  ]

  /** Tints for the maker's other apps until their own icons ship with them. */
  const makers = [
    { key: 'raff', tint: '#7a5c3e' },
    { key: 'luma', tint: '#c9862b' },
    { key: 'nafidh', tint: '#3e6b5a' },
  ]

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
  {#each links as link, index (link.key)}
    <button class="link" type="button" onclick={() => openExternal(link.url)}>
      <FormRow first={index === 0} title={t(`settings.about.${link.key}`)}>
        {#if link.value}<span class="value">{link.value}</span>{/if}
        <span class="affordance"><Icon name="external" size={16} /></span>
      </FormRow>
    </button>
  {/each}
  <button class="link" type="button" onclick={welcome}>
    <FormRow title={t('settings.about.showWelcome')}>
      <!-- Forward: right in English, mirrored to the left in Arabic. -->
      <span class="affordance"><Icon name="chevronRight" size={16} flip /></span>
    </FormRow>
  </button>
</GroupCard>

<GroupCard title={t('settings.about.fromSameMaker')}>
  <ul class="makers">
    {#each makers as maker (maker.key)}
      {@const name = t(`settings.about.makerApps.${maker.key}`)}
      <li>
        <span class="maker-icon" style:background={maker.tint} aria-hidden="true">{name.charAt(0)}</span>
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
    gap: 4px;
    padding-block: 4px;
    text-align: center;
  }

  img {
    display: block;
  }

  h1 {
    font-size: var(--size-title);
    line-height: var(--leading-title);
    font-weight: 700;
  }

  .version,
  .tagline {
    margin: 0;
    color: var(--text-secondary);
  }

  .version {
    font-size: var(--size-small);
    line-height: var(--leading-small);
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
    box-shadow: inset 0 0 0 var(--focus-width) var(--focus-ring);
  }

  .value {
    color: var(--text-secondary);
    font-size: 13px;
  }

  .affordance {
    display: grid;
    color: var(--text-tertiary);
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
    gap: 10px;
    padding: 12px 16px;
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
    border-radius: var(--radius-tab);
    color: #fff;
    font-family: var(--font-heading);
    font-size: 15px;
    font-weight: 700;
    line-height: 1;
  }

  footer {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    text-align: center;
  }

  footer p {
    margin: 0;
  }

  .made-by {
    display: flex;
    gap: 6px;
    font-size: var(--size-small);
    line-height: var(--leading-small);
    color: var(--text-secondary);
  }

  .made-by strong {
    color: var(--text-primary);
  }

  .repo {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 0 4px;
    border: none;
    border-radius: var(--radius-keycap);
    background: none;
    color: var(--accent-primary);
    font-size: var(--size-caption);
    line-height: var(--leading-small);
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
