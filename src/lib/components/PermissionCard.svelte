<script lang="ts">
  import Button from './Button.svelte'
  import Icon from './Icon.svelte'
  import { app, requestPermission, t } from '../state.svelte'

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
      <span class="spinner"></span>
    {/if}
  </span>
  <p class="text">
    {#if state === 'granted'}
      {t('permission.granted')}
    {:else if state === 'missing'}
      {t('permission.missing')}
    {:else}
      {t('permission.checking')}
    {/if}
  </p>
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
    padding: 12px 16px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-card);
    background: var(--bg-surface);
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
    background: color-mix(in srgb, var(--state-success) 12%, transparent);
    color: var(--state-success);
  }

  .missing .badge {
    background: color-mix(in srgb, var(--state-warning) 12%, transparent);
    color: var(--state-warning);
  }

  .checking .badge {
    background: var(--bg-sunken);
    color: var(--text-secondary);
  }

  .text {
    margin: 0;
    flex: 1;
    font-size: var(--size-body);
    line-height: var(--leading-body);
  }

  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid var(--border-subtle);
    border-top-color: var(--text-secondary);
    border-radius: 50%;
    animation: spin 700ms linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
