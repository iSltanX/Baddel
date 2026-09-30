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
  import Keycap from './lib/components/Keycap.svelte'
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

  function imageLine(i: ReportImage, withName: boolean): string {
    const format = i.mime === 'image/jpeg' ? 'JPEG' : 'PNG'
    const parts = withName ? [i.name, i.mime] : [format]
    return [...parts, size(i.bytes), `${i.width}×${i.height}`].join(' · ')
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

        <section class="group">
          <h2><label for="description">{t('report.description')}</label></h2>
          <textarea
            id="description"
            class="field"
            dir={description ? 'auto' : undefined}
            rows="5"
            maxlength={MAX + 200}
            placeholder={t('report.descriptionPlaceholder')}
            bind:value={description}
          ></textarea>
          <div class="note">
            <p>{t('report.descriptionHint')}</p>
            <span class="counter" class:over={count > MAX}>{t('report.counter', { count, max: MAX })}</span>
          </div>
        </section>

        <section class="group">
          <h2>{t('report.image')}</h2>
          <div class="card">
            {#if image}
              <div class="image-row">
                <img class="thumb" src={image.thumbnail} alt="" />
                <div class="image-text">
                  <span>{t('report.attached')}</span>
                  <span class="meta ltr">{imageLine(image, false)}</span>
                </div>
                <Button variant="plain" onclick={removeImage}>{t('report.remove')}</Button>
              </div>
            {:else}
              <div class="image-row">
                <span class="paste">
                  {t('report.pasteHint')}
                  <span class="keys ltr"><Keycap label="⌘" /><Keycap label="V" /></span>
                </span>
                <Button onclick={() => attach(pickReportImage)}>{t('report.chooseImage')}</Button>
              </div>
            {/if}
          </div>
          {#if image}
            <p class="warning"><Icon name="warning" size={16} />{t('report.imageWarning')}</p>
          {/if}
          {#if imageError}
            <p class="warning" role="alert"><Icon name="warning" size={16} />{imageError}</p>
          {/if}
          <div class="note"><p>{t('report.imageLimits')}</p></div>
        </section>
      {:else if screen === 'preview' && preview}
        <div class="lead">
          <p class="strong">{t('report.previewIntro')}</p>
          <p class="secondary">{t('report.previewDestination')}</p>
        </div>

        <GroupCard title={t('report.groups.report')}>
          <FormRow first title={t('report.fields.type')}>
            <span class="value ltr">{preview.kind}{preview.category ? ` · ${preview.category}` : ''}</span>
          </FormRow>
          <FormRow title={t('report.fields.app')}><span class="value ltr">baddel {preview.appVersion}</span></FormRow>
          <FormRow title={t('report.fields.system')}>
            <span class="value ltr">macos {preview.osVersion} · {preview.arch}</span>
          </FormRow>
          <FormRow title={t('report.fields.locale')}><span class="value ltr">{preview.locale}</span></FormRow>
          {#if preview.test}
            <FormRow title={t('report.fields.test')}><span class="value ltr">true</span></FormRow>
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
                <span class="meta ltr grow">{imageLine(preview.image, true)}</span>
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
          <span class="result-icon warning-tone"><Icon name="warning" size={44} /></span>
          <h1>{t('report.failedTitle')}</h1>
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
      <Button onclick={() => copy('report')}>{copied === 'report' ? t('report.copied') : t('report.copyReport')}</Button>
      {#if failure?.kind !== 'rejected'}
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
    gap: 16px;
    padding: var(--window-pad);
  }

  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 14px var(--window-pad);
    border-top: 1px solid var(--border-subtle);
  }

  p {
    margin: 0;
  }

  .intro,
  .secondary {
    color: var(--text-secondary);
    font-size: var(--size-small);
    line-height: var(--leading-small);
  }

  .lead {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .strong {
    font-weight: 700;
  }

  /* The same label, card and footnote as GroupCard, around content that is not rows. */
  .group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  h2 {
    min-height: 22px;
    padding-inline: 16px;
    font-family: var(--font-body);
    font-size: var(--size-small);
    line-height: 22px;
    font-weight: 700;
    color: var(--text-secondary);
  }

  .card {
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-card);
    background: var(--bg-surface);
    overflow: hidden;
  }

  .sunken {
    background: var(--bg-sunken);
  }

  .note {
    display: flex;
    align-items: baseline;
    gap: 12px;
    padding-inline: 16px;
    color: var(--text-secondary);
    font-size: var(--size-small);
    line-height: var(--leading-small);
  }

  .note p {
    flex: 1;
  }

  .counter {
    flex: none;
    color: var(--text-tertiary);
  }

  .counter.over {
    color: var(--state-danger);
  }

  .field {
    display: block;
    width: 100%;
    min-height: 120px;
    padding: 8px 12px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-control);
    background: var(--bg-surface);
    box-shadow: var(--shadow-control);
    font-size: var(--size-body);
    line-height: var(--leading-body);
    resize: none;
    user-select: text;
    cursor: text;
  }

  .field::placeholder {
    color: var(--text-tertiary);
  }

  .field:focus-visible {
    box-shadow: 0 0 0 var(--focus-width) var(--focus-ring);
  }

  .image-row {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: var(--row-height);
    padding: 10px 16px;
  }

  .thumb {
    width: 56px;
    height: 40px;
    flex: none;
    object-fit: cover;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-keycap);
    background: var(--bg-sunken);
  }

  .image-text {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
  }

  .meta {
    color: var(--text-secondary);
    font-family: var(--font-latin);
    font-size: var(--size-caption);
    line-height: var(--leading-small);
  }

  .grow {
    flex: 1;
  }

  .paste {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 8px;
    color: var(--text-secondary);
    font-size: var(--size-small);
  }

  .keys {
    display: inline-flex;
    gap: 3px;
  }

  .warning {
    display: flex;
    align-items: center;
    gap: 6px;
    padding-inline: 16px;
    color: var(--state-warning-text);
    font-size: var(--size-small);
    line-height: var(--leading-small);
  }

  .warning :global(svg) {
    color: var(--state-warning);
  }

  .value {
    color: var(--text-secondary);
    font-family: var(--font-latin);
    font-size: 13px;
  }

  .description {
    padding: 10px 16px;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    user-select: text;
  }

  pre {
    margin: 0;
    padding: 10px 16px;
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
    gap: 8px;
    padding: 10px 14px;
    border-radius: var(--radius-card);
    background: var(--accent-soft);
    font-size: var(--size-small);
    line-height: var(--leading-small);
  }

  .promise :global(svg) {
    color: var(--accent-primary);
  }

  .result {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 16px 20px 12px;
    text-align: center;
  }

  .result-icon {
    display: grid;
  }

  .success {
    color: var(--state-success);
  }

  .warning-tone {
    color: var(--state-warning);
  }

  h1 {
    font-size: var(--size-title);
    line-height: var(--leading-title);
    font-weight: 700;
  }

  .number-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .number {
    padding: 4px 18px;
    border-radius: var(--radius-card);
    background: var(--accent-soft);
    color: var(--accent-primary);
    font-family: var(--font-latin);
    font-size: var(--size-sample);
    line-height: var(--leading-sample);
    font-weight: 600;
    user-select: text;
  }

  .caption {
    color: var(--text-tertiary);
    font-size: var(--size-small);
    line-height: var(--leading-small);
  }
</style>
