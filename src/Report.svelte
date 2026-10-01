<script lang="ts">
  /**
   * The problem-report window: form → preview → result. It holds no report state that
   * matters beyond the screen it shows — the image and the payload stay in Rust, which
   * builds the preview from the very payload it sends, and sends only after "Send".
   */
  import Button from './lib/components/Button.svelte'
  import FormRow from './lib/components/FormRow.svelte'
  import GroupCard from './lib/components/GroupCard.svelte'
  import Icon from './lib/components/Icon.svelte'
  import ReportDescription from './lib/components/ReportDescription.svelte'
  import ReportImageField from './lib/components/ReportImageField.svelte'
  import Select from './lib/components/Select.svelte'
  import {
    clearReportImage,
    closeWindow,
    copyReport,
    copyReportNumber,
    pasteReportImage,
    pickReportImage,
    previewReport,
    reveal,
    sendReport,
    setContentHeight,
    setWindowTitle,
    t,
    type Problem,
    type ReportImage,
    type ReportPreview,
    type SendFailure,
  } from './lib/state.svelte'

  /** Keep in step with `report::MAX_DESCRIPTION`. */
  const MAX = 1000
  /** Beyond this the window stops growing and the content scrolls. */
  const MAX_HEIGHT = 720
  const PROBLEMS: Problem[] = ['wrong-conversion', 'no-effect', 'undo', 'shortcut', 'crash', 'suggestion', 'other']

  type Screen = 'form' | 'preview' | 'sent' | 'failed'
  let screen = $state<Screen>('form')
  let problem = $state<Problem>('wrong-conversion')
  let description = $state('')
  let image = $state<ReportImage | null>(null)
  let imageError = $state<string | null>(null)
  let preview = $state<ReportPreview | null>(null)
  let reportId = $state<number | null>(null)
  let failure = $state<SendFailure | null>(null)
  let busy = $state(false)
  let copied = $state<'number' | 'report' | null>(null)

  const count = $derived([...description.trim()].length)
  const ready = $derived(count > 0 && count <= MAX)
  const options = $derived(PROBLEMS.map((value) => ({ value, label: t(`report.problems.${value}`) })))

  function size(bytes: number): string {
    return bytes < 1024 * 1024 ? `${Math.max(1, Math.round(bytes / 1024))} KB` : `${(bytes / 1024 / 1024).toFixed(1)} MB`
  }

  /** The preview names the image exactly as it is sent: file name, type, size, dimensions. */
  function imageLine(i: ReportImage): string {
    return [i.name, i.mime, size(i.bytes), `${i.width}×${i.height}`].join(' · ')
  }

  async function attach(task: () => Promise<ReportImage | null>) {
    imageError = null
    try {
      const picked = await task()
      if (picked) image = picked
    } catch (code) {
      imageError = t(`report.imageErrors.${String(code)}`)
    }
  }

  async function removeImage() {
    image = null
    imageError = null
    await clearReportImage()
  }

  /** ⌘V with an image on the pasteboard attaches it, wherever the focus is. */
  function onpaste(event: ClipboardEvent) {
    if (screen !== 'form') return
    const types = [...(event.clipboardData?.types ?? [])]
    if (types.some((type) => type.startsWith('image/') || type === 'Files')) {
      event.preventDefault()
      void attach(pasteReportImage)
    }
  }

  function onkeydown(event: KeyboardEvent) {
    const inField = event.target instanceof HTMLTextAreaElement || event.target instanceof HTMLSelectElement
    if (screen === 'form' && event.metaKey && event.key.toLowerCase() === 'v' && !inField) {
      event.preventDefault()
      void attach(pasteReportImage)
    }
  }

  async function toPreview() {
    if (!ready) return
    preview = await previewReport(problem, description)
    screen = 'preview'
  }

  async function send() {
    busy = true
    try {
      reportId = await sendReport()
      screen = 'sent'
    } catch (error) {
      failure = (error as SendFailure) ?? { kind: 'retry' }
      screen = 'failed'
    } finally {
      busy = false
    }
  }

  async function copy(what: 'number' | 'report') {
    if (what === 'number' && reportId !== null) await copyReportNumber(reportId)
    if (what === 'report') await copyReport()
    copied = what
    setTimeout(() => (copied = null), 1600)
  }

  const rejected = $derived(failure?.kind === 'rejected')

  const failureText = $derived.by(() => {
    if (!failure) return ''
    if (failure.kind === 'rate-limited') return t('report.failedRateLimited', { minutes: failure.minutes })
    if (failure.kind === 'rejected') return t('report.failedRejected', { reason: failure.reason })
    return t('report.failedRetry')
  })

  $effect(() => {
    void setWindowTitle(t('report.title'))
  })

  // The window is as tall as the screen needs, up to MAX_HEIGHT; past that the content scrolls.
  let inner = $state<HTMLElement>()
  let footer = $state<HTMLElement>()
  $effect(() => {
    if (!inner || !footer) return
    const measure = () => void setContentHeight(Math.min(MAX_HEIGHT, Math.ceil(inner!.offsetHeight + footer!.offsetHeight + 1)))
    const observer = new ResizeObserver(measure)
    observer.observe(inner)
    observer.observe(footer)
    return () => observer.disconnect()
  })

  $effect(() => {
    void reveal()
  })
</script>

<svelte:window {onpaste} {onkeydown} />

<div class="window">
  <main>
    <div class="inner" bind:this={inner}>
      {#if screen === 'form'}
        <p class="intro">{t('report.intro')}</p>

        <GroupCard>
          <FormRow first title={t('report.problemType')} labelFor="problem">
            <Select id="problem" value={problem} {options} onchange={(value) => (problem = value as Problem)} />
          </FormRow>
        </GroupCard>

        <ReportDescription bind:value={description} max={MAX} />

        <ReportImageField {image} error={imageError} onchoose={() => attach(pickReportImage)} onremove={removeImage} />
      {:else if screen === 'preview' && preview}
        <div class="lead">
          <p class="strong">{t('report.previewIntro')}</p>
          <p class="secondary">{t('report.previewDestination')}</p>
        </div>

        <GroupCard title={t('report.groups.report')}>
          <FormRow dense first title={t('report.fields.type')}>
            <span class="value ltr">{preview.kind}{preview.category ? ` · ${preview.category}` : ''}</span>
          </FormRow>
          <FormRow dense title={t('report.fields.app')}><span class="value ltr">baddel {preview.appVersion}</span></FormRow>
          <FormRow dense title={t('report.fields.system')}>
            <span class="value ltr">macos {preview.osVersion} · {preview.arch}</span>
          </FormRow>
          <FormRow dense title={t('report.fields.locale')}><span class="value ltr">{preview.locale}</span></FormRow>
          {#if preview.test}
            <FormRow dense title={t('report.fields.test')}><span class="value ltr">true</span></FormRow>
          {/if}
        </GroupCard>

        <section class="group">
          <h2>{t('report.groups.description')}</h2>
          <div class="card sunken"><p class="description" dir="auto">{preview.description}</p></div>
        </section>

        {#if preview.image}
          <section class="group">
            <h2>{t('report.groups.image')}</h2>
            <div class="card">
              <div class="image-row">
                <img class="thumb" src={preview.image.thumbnail} alt="" />
                <span class="meta ltr">{imageLine(preview.image)}</span>
              </div>
            </div>
          </section>
        {/if}

        <section class="group">
          <h2>{t('report.groups.diagnostics')}</h2>
          <div class="card sunken"><pre dir="ltr">{preview.diagnostics}</pre></div>
        </section>

        <p class="promise"><Icon name="shield" size={16} />{t('report.onlyThis')}</p>
      {:else if screen === 'sent' && reportId !== null}
        <div class="result">
          <span class="result-icon success"><Icon name="checkCircle" size={44} /></span>
          <h1>{t('report.sentTitle')}</h1>
          <p class="secondary">{t('report.sentBody')}</p>
          <div class="number-row">
            <span class="number ltr">#{reportId}</span>
            <Button onclick={() => copy('number')}>{copied === 'number' ? t('report.copied') : t('report.copyNumber')}</Button>
          </div>
          <p class="caption">{t('report.keepNumber')}</p>
        </div>
      {:else if screen === 'failed'}
        <div class="result">
          <span class="result-icon" class:warning-tone={!rejected} class:danger-tone={rejected}>
            <Icon name="warning" size={44} />
          </span>
          <h1>{rejected ? t('report.rejectedTitle') : t('report.failedTitle')}</h1>
          <p class="secondary">{failureText}</p>
          <p class="caption">{t('report.copyHint')}</p>
        </div>
      {/if}
    </div>
  </main>

  <footer bind:this={footer}>
    {#if screen === 'form'}
      <Button onclick={closeWindow}>{t('report.cancel')}</Button>
      <Button variant="primary" disabled={!ready} onclick={toPreview}>{t('report.next')}</Button>
    {:else if screen === 'preview'}
      <Button disabled={busy} onclick={() => (screen = 'form')}>{t('report.back')}</Button>
      <Button variant="primary" loading={busy} onclick={send}>{busy ? t('report.sending') : t('report.send')}</Button>
    {:else if screen === 'sent'}
      <Button variant="primary" onclick={closeWindow}>{t('report.done')}</Button>
    {:else if screen === 'failed'}
      <!-- A refused report cannot be retried: copying it is the one way left, so it leads. -->
      <Button variant={rejected ? 'primary' : 'secondary'} disabled={busy} onclick={() => copy('report')}>
        {copied === 'report' ? t('report.copied') : t('report.copyReport')}
      </Button>
      {#if !rejected}
        <Button variant="primary" loading={busy} onclick={send}>{t('report.retry')}</Button>
      {/if}
    {/if}
  </footer>
</div>

<style>
  .window {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  main {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }

  .inner {
    display: flex;
    flex-direction: column;
    gap: var(--space-16);
    padding: var(--space-20);
  }

  footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--space-8);
    min-height: 56px;
    padding: 0 var(--space-20);
    border-top: 1px solid var(--border-subtle);
  }

  .intro,
  .secondary {
    color: var(--text-secondary);
  }

  .intro {
    font-size: var(--size-label);
    line-height: var(--leading-label);
  }

  .lead {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .lead .secondary {
    font-size: var(--size-label);
    line-height: var(--leading-label);
  }

  .strong {
    font-weight: 700;
  }

  /* The same label, card and footnote as GroupCard, around content that is not rows. */
  .group {
    display: flex;
    flex-direction: column;
    gap: var(--space-8);
  }

  h2 {
    min-height: 22px;
    padding-inline: var(--space-16);
    font-family: var(--font-body);
    font-size: var(--size-label);
    line-height: 22px;
    font-weight: 700;
    color: var(--text-secondary);
  }

  .card {
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    background: var(--bg-surface);
    overflow: hidden;
  }

  .sunken {
    background: var(--bg-surface-secondary);
  }

  .image-row {
    display: flex;
    align-items: center;
    gap: var(--space-12);
    min-height: 60px;
    padding: var(--space-8) var(--space-16);
  }

  .thumb {
    width: 56px;
    height: 40px;
    flex: none;
    object-fit: cover;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-xs);
    background: var(--bg-surface-secondary);
  }

  /* A Latin line inside an RTL card: it hugs its text from the reading side. */
  .meta {
    flex: 1;
    min-width: 0;
    color: var(--text-secondary);
    font-family: var(--font-latin);
    font-size: var(--size-latin-small);
    line-height: var(--leading-label);
    text-align: end;
  }

  :global([dir='ltr']) .meta {
    text-align: start;
  }

  .value {
    color: var(--text-secondary);
    font-family: var(--font-latin);
    font-size: var(--size-latin);
  }

  .description {
    padding: var(--space-12) var(--space-16);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    user-select: text;
  }

  pre {
    margin: 0;
    padding: var(--space-12) var(--space-16);
    font-family: var(--font-mono);
    font-size: var(--size-caption);
    line-height: var(--leading-caption);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    text-align: left;
    user-select: text;
  }

  .promise {
    display: flex;
    align-items: center;
    gap: var(--space-8);
    padding: var(--space-12) var(--space-16);
    border-radius: var(--radius-md);
    background: var(--bg-selected);
    font-size: var(--size-label);
    line-height: var(--leading-label);
  }

  .promise :global(svg) {
    color: var(--text-brand);
  }

  .result {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-12);
    padding: var(--space-32) var(--space-48);
    text-align: center;
  }

  .result-icon {
    display: grid;
  }

  .success {
    color: var(--success);
  }

  .warning-tone {
    color: var(--warning);
  }

  .danger-tone {
    color: var(--danger);
  }

  h1 {
    font-size: var(--size-title);
    line-height: var(--leading-title);
  }

  .number-row {
    display: flex;
    align-items: center;
    gap: var(--space-12);
  }

  .number {
    display: flex;
    align-items: center;
    min-height: 44px;
    padding: 0 var(--space-16);
    border-radius: var(--radius-md);
    background: var(--bg-selected);
    color: var(--text-brand);
    font-family: var(--font-latin);
    font-size: 21px;
    line-height: 32px;
    font-weight: 600;
    user-select: text;
  }

  .caption {
    color: var(--text-tertiary);
    font-size: var(--size-label);
    line-height: var(--leading-label);
  }
</style>
