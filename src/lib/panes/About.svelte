<script lang="ts">
  import Button from '../components/Button.svelte'
  import Icon from '../components/Icon.svelte'
  import { appVersion, closeWindow, openExternal, openOnboarding, t } from '../state.svelte'
  import iconUrl from '../../assets/app-icon.png'

  const REPOSITORY = 'https://github.com/iSltanX/Baddel'

  let version = $state('')
  $effect(() => {
    void appVersion().then((value) => (version = value))
  })

  const links = [
    { key: 'sourceOnGithub', url: REPOSITORY },
    { key: 'reportIssue', url: `${REPOSITORY}/issues` },
    { key: 'license', url: `${REPOSITORY}/blob/main/LICENSE` },
  ]

  async function welcome() {
    await openOnboarding()
    await closeWindow()
  }
</script>

<div class="about">
  <img src={iconUrl} alt="" width="96" height="96" />
  <h1>{t('settings.about.appName')}</h1>
  <p class="version">{t('settings.about.version', { version })}</p>
  <p class="tagline">{t('settings.about.tagline')}</p>

  <div class="links">
    {#each links as link (link.key)}
      <Button variant="plain" onclick={() => openExternal(link.url)}>
        {t(`settings.about.${link.key}`)}
        <Icon name="external" size={13} />
      </Button>
    {/each}
  </div>

  <Button variant="plain" onclick={welcome}>{t('settings.about.showWelcome')}</Button>

  <div class="maker">
    <span class="maker-label">{t('settings.about.fromSameMaker')}</span>
    <ul>
      {#each ['raff', 'luma', 'nafidh'] as key (key)}
        <li>
          <Icon name="appGeneric" size={16} />
          {t(`settings.about.makerApps.${key}`)}
        </li>
      {/each}
    </ul>
  </div>
</div>

<style>
  .about {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    text-align: center;
  }

  img {
    border-radius: 18px;
    margin-bottom: 6px;
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
    font-size: var(--size-small);
    line-height: var(--leading-small);
  }

  .tagline {
    color: var(--text-primary);
    font-size: var(--size-body);
    margin-bottom: 8px;
  }

  .links {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 4px;
  }

  .maker {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    width: 100%;
    margin-top: 10px;
    padding-top: 14px;
    border-top: 1px solid var(--border-subtle);
  }

  .maker-label {
    color: var(--text-secondary);
    font-size: var(--size-small);
  }

  ul {
    display: flex;
    gap: 18px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-secondary);
    font-size: var(--size-small);
  }
</style>
