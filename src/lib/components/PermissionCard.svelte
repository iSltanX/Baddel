<script lang="ts">
  import Button from './Button.svelte'
  import Icon from './Icon.svelte'
  import { app, requestPermission, t } from '../state.svelte'

  /**
   * `PermissionCard` in 02 — Components: the Accessibility permission in one of three
   * states. Granted is a quiet status row; missing turns amber and is the only state
   * with an action.
   */
  interface Props {
    /**
     * We do not know yet, and say so rather than guessing: before the first check, or
     * while System Settings is open and Baddel watches for the switch.
     */
    checking?: boolean
    /** Called after the card's own button has opened System Settings. */
    onrequest?: () => void
  }

  const { checking = false, onrequest }: Props = $props()
  const state = $derived(app.permission ? 'granted' : checking ? 'checking' : 'missing')

  async function request() {
    await requestPermission()
    onrequest?.()
  }
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
    <Button variant="primary" onclick={request}>
      {t('permission.openSystemSettings')}
    </Button>
  {/if}
</div>

<style>
  .card {
    display: flex;
    align-items: center;
    gap: var(--space-12);
    min-height: 66px;
    padding: var(--space-12) var(--space-16);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    background: var(--bg-surface);
  }

  .missing {
    border-color: var(--warning);
    background: var(--warning-soft);
  }

  .badge {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    flex: none;
    border-radius: var(--radius-full);
  }

  /* State is carried by the icon as well as the colour, never by colour alone. */
  .granted .badge {
    background: var(--success-soft);
    color: var(--success);
  }

  .missing .badge {
    background: var(--bg-surface);
    color: var(--warning);
  }

  .checking .badge {
    background: var(--bg-surface-secondary);
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
    font-size: var(--size-label);
    line-height: var(--leading-label);
  }

  .missing .description {
    color: var(--warning-text);
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
