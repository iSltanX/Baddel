<script lang="ts">
  import Button from './Button.svelte'
  import Icon from './Icon.svelte'
  import { app, requestPermission, t } from '../state.svelte'

  /**
   * The Accessibility permission, in one of three states. Granted is a quiet
   * status row; missing turns amber and is the only state with an action.
   */
  interface Props {
    /** Before the first check we do not know yet, and say so rather than guessing. */
    checking?: boolean
  }

  const { checking = false }: Props = $props()
  const state = $derived(checking ? 'checking' : app.permission ? 'granted' : 'missing')
</script>

<div class="card {state}" role="status">
  <span class="badge">
    {#if state === 'granted'}
      <Icon name="checkCircle" size={18} />
    {:else if state === 'missing'}
      <Icon name="warning" size={18} />
    {:else}
      <span class="spinner"><Icon name="spinner" size={18} /></span>
    {/if}
  </span>
  <div class="text">
    <span class="title">{t(`permission.${state}`)}</span>
    <span class="description">{t(`permission.${state}Description`)}</span>
  </div>
  {#if state === 'missing'}
    <Button variant="primary" onclick={requestPermission}>
      {t('permission.openSystemSettings')}
    </Button>
  {/if}
</div>

<style>
  .card {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 60px;
    padding: 12px 16px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-card);
    background: var(--bg-surface);
  }

  .missing {
    border-color: color-mix(in srgb, var(--state-warning) 45%, transparent);
    background: var(--state-warning-soft);
  }

  .badge {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    flex: none;
    border-radius: 50%;
  }

  /* State is carried by the icon as well as the colour, never by colour alone. */
  .granted .badge {
    background: var(--state-success-soft);
    color: var(--state-success);
  }

  .missing .badge {
    background: var(--bg-surface);
    color: var(--state-warning);
  }

  .checking .badge {
    background: var(--bg-sunken);
    color: var(--text-secondary);
  }

  .text {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
    text-align: start;
  }

  .title {
    font-size: var(--size-body);
    line-height: var(--leading-body);
    font-weight: 700;
  }

  .description {
    color: var(--text-secondary);
    font-size: var(--size-small);
    line-height: var(--leading-small);
  }

  .missing .description {
    color: var(--state-warning-text);
  }

  .spinner {
    display: grid;
    animation: spin 900ms linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
