<script setup>
import { computed, onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { mountWhipOverlay } from './whipOverlay'
import logoMark from './assets/openbad-mark.svg'

const LANGUAGE_ZH_CN = 'zh-CN'
const LANGUAGE_EN_US = 'en-US'

const DEFAULT_SETTINGS = Object.freeze({
  promptText: '快点，加油',
  showWhipShortcut: 'Shift+1',
  sendPromptShortcut: 'Shift+2',
  hideWhipShortcut: 'Shift+3',
  language: LANGUAGE_ZH_CN,
  crackSendMode: 'selectedInput',
  targets: [],
})

const MESSAGES = {
  [LANGUAGE_ZH_CN]: {
    brand: {
      eyebrow: 'OpenBad',
      product: '桌面挥鞭助手',
      caption: '为 Codex / Tauri 桌面流程准备的跟随式鞭子 companion。',
      title: 'Whip Follow Mode',
      lead:
        '激活快捷键后，鞭子会立刻跟随鼠标，不再需要按住左键。你可以提前配置命中区域和对应文本，让挥鞭直接落到指定输入框并提交。',
      logoAlt: 'OpenBad 标志',
    },
    language: {
      label: '界面语言',
      hint: '立即切换设置界面、托盘和菜单栏文案。',
    },
    summary: {
      aria: '当前工作流摘要',
      chips: ['激活后立即跟随鼠标', '支持独立退出快捷键', '支持目标区域文本映射'],
    },
    cards: {
      fallback: {
        kicker: 'Fallback Prompt',
        title: '默认发送文字',
        copy:
          '当挥鞭没有命中任何预设区域时，会退回这里的默认文本。手动“发送文字并回车”快捷键也使用这一项。',
      },
      shortcuts: {
        kicker: 'Shortcuts',
        title: '全局快捷键',
        copy: '聚焦输入框后直接按组合键录制。三个快捷键不能重复。',
      },
      mode: {
        kicker: 'Crack Mode',
        title: '挥鞭时把文字发到哪里',
        copy:
          '“之前选中的输入框”模式会在挥鞭时切回你之前选中的文本框并输入。“预设区域”模式则按命中的区域中心点击并发送对应文本。',
      },
      targets: {
        kicker: 'Target Zones',
        title: '预设文本框区域',
        copy:
          '坐标使用当前屏幕百分比。挥鞭命中区域后，会点击该区域中心并发送对应文本；overlay 上会显示这些区域轮廓。',
      },
      workflow: {
        kicker: 'Workflow',
        title: '新的操作逻辑',
      },
    },
    fields: {
      promptText: '默认内容',
      showShortcut: '呼出鞭子并开始跟随',
      sendShortcut: '发送默认文字并回车',
      hideShortcut: '退出鞭子模式',
      targetName: '区域名称',
      targetPromptText: '命中后发送的文本',
      x: 'X (%)',
      y: 'Y (%)',
      width: 'Width (%)',
      height: 'Height (%)',
    },
    placeholders: {
      promptText: '例如：快点，加油',
      showShortcut: 'Shift+1',
      sendShortcut: 'Shift+2',
      hideShortcut: 'Shift+3',
      targetName: '例如：Claude Input',
      targetPromptText: '留空时会使用默认文字',
    },
    help: {
      fallback: '留空时会回退到默认值“快点，加油”。',
      hideShortcut:
        '单独按一次 Esc 也会让当前 overlay 直接退出；这里只是额外的全局退出快捷键。',
      targetsInactive:
        '当前模式是“之前选中的输入框”。下面这些区域配置会保留，但挥鞭时不会启用，除非你把模式切回“预设区域”。',
      noTargets: '还没有目标区域。先添加一个文本框区域，再设置它的屏幕百分比和对应文本。',
      targetExample: '例如 Claude 输入框可以先试 x 14 / y 78 / width 72 / height 12。',
      coordinateHint: '保存后再次呼出鞭子，就能在 overlay 上看到目标框和命中特效。',
    },
    permissions: {
      kicker: 'Permissions',
      title: 'macOS 权限说明',
      copy: '如果挥鞭后不能自动切回输入框、不能输入文本，基本就是这里的权限没有放开。',
      items: [
        '辅助功能：必须开启，应用要靠它点击别的输入框并发送鼠标、键盘事件。',
        '当前版本已经改成原生切回前一个应用，不再依赖 System Events。',
        '开发模式下如果是从 Terminal、iTerm 或 Warp 启动，也要给对应终端开启辅助功能权限。',
      ],
    },
    modes: {
      selectedInput: '之前选中的输入框',
      targetZones: '预设区域',
    },
    targetActions: {
      add: '添加目标区域',
      duplicate: '复制',
      remove: '删除',
    },
    workflow: {
      steps: [
        {
          title: '按呼出快捷键',
          body: '鞭子会直接出现在当前鼠标附近，并持续跟随鼠标，不需要按住左键。',
        },
        {
          title: '快速挥鞭',
          body:
            '如果当前模式是“之前选中的输入框”，它会切回之前选中的文本框输入。如果当前模式是“预设区域”，则按命中的区域中心发送对应文本。',
        },
        {
          title: '自动点击并提交',
          body: '命中后会点击该区域中心，输入目标文本，再自动按一次 Return。',
        },
        {
          title: '按退出快捷键或 Esc',
          body: '随时退出 overlay，避免整屏持续处于鞭子跟随模式。',
        },
      ],
      noteTitle: '坐标说明',
      noteLines: [
        'x / y 代表区域左上角',
        'width / height 代表区域尺寸',
        '单位统一是当前屏幕百分比',
      ],
    },
    footer: {
      restore: '恢复默认',
      save: '保存设置',
      saving: '保存中...',
      loading: '加载中...',
    },
    status: {
      shortcutCleared: '已清空该快捷键，保存后会恢复默认值。',
      defaultsRestored: '已恢复默认值，点击保存后生效。',
      saved: '设置已保存并立即生效。',
      loadFailed: '加载设置失败',
      saveFailed: '保存设置失败',
    },
  },
  [LANGUAGE_EN_US]: {
    brand: {
      eyebrow: 'OpenBad',
      product: 'Desktop Whip Companion',
      caption: 'A follow-mode whip companion tuned for Codex and Tauri desktop workflows.',
      title: 'Whip Follow Mode',
      lead:
        'After the activation shortcut, the whip immediately follows the cursor without holding the left mouse button. You can pre-map hit zones and prompt text so a crack lands on the right input and submits it.',
      logoAlt: 'OpenBad logo',
    },
    language: {
      label: 'Language',
      hint: 'Switch the settings UI, tray labels, and app menu copy instantly.',
    },
    summary: {
      aria: 'Current workflow summary',
      chips: [
        'Follow mouse immediately after activation',
        'Dedicated exit shortcut supported',
        'Per-target prompt mapping supported',
      ],
    },
    cards: {
      fallback: {
        kicker: 'Fallback Prompt',
        title: 'Default prompt text',
        copy:
          'When a crack misses every preset zone, it falls back to this prompt. The manual “send prompt and return” shortcut also uses this value.',
      },
      shortcuts: {
        kicker: 'Shortcuts',
        title: 'Global shortcuts',
        copy: 'Focus each field and press the combination directly. The three shortcuts must stay unique.',
      },
      mode: {
        kicker: 'Crack Mode',
        title: 'Where a whip crack sends text',
        copy:
          '“Previously selected input” switches back to the text field you were using and types there. “Preset target zones” clicks the center of the matched zone and sends that zone prompt.',
      },
      targets: {
        kicker: 'Target Zones',
        title: 'Preset text input zones',
        copy:
          'Coordinates use the current screen percentage. When a crack lands inside a zone, the app clicks the zone center and sends its prompt. The overlay also renders those zone outlines.',
      },
      workflow: {
        kicker: 'Workflow',
        title: 'Updated interaction flow',
      },
    },
    fields: {
      promptText: 'Default text',
      showShortcut: 'Show whip and start following',
      sendShortcut: 'Send default text and press Return',
      hideShortcut: 'Exit whip mode',
      targetName: 'Zone name',
      targetPromptText: 'Prompt text after a hit',
      x: 'X (%)',
      y: 'Y (%)',
      width: 'Width (%)',
      height: 'Height (%)',
    },
    placeholders: {
      promptText: 'Example: 快点，加油',
      showShortcut: 'Shift+1',
      sendShortcut: 'Shift+2',
      hideShortcut: 'Shift+3',
      targetName: 'Example: Claude Input',
      targetPromptText: 'Falls back to the default text when empty',
    },
    help: {
      fallback: 'An empty value falls back to 快点，加油.',
      hideShortcut:
        'Pressing Esc once also exits the current overlay; this field is only the extra global escape shortcut.',
      targetsInactive:
        'The current mode is “Previously selected input”. These zone settings are preserved, but they stay inactive until you switch back to “Preset target zones”.',
      noTargets: 'No target zones yet. Add one first, then set its screen percentages and prompt text.',
      targetExample: 'For a Claude input, start with x 14 / y 78 / width 72 / height 12.',
      coordinateHint: 'After saving, summon the whip again to see the zone outlines and hit effects on the overlay.',
    },
    permissions: {
      kicker: 'Permissions',
      title: 'macOS permission checklist',
      copy: 'If the whip can no longer switch back to the input box or type text, the missing permission is usually here.',
      items: [
        'Accessibility: required so the app can click external inputs and post mouse or keyboard events.',
        'This build now switches back to the previous app natively and no longer depends on System Events.',
        'If you launch from Terminal, iTerm, or Warp in dev mode, grant Accessibility to that terminal as well.',
      ],
    },
    modes: {
      selectedInput: 'Previously selected input',
      targetZones: 'Preset target zones',
    },
    targetActions: {
      add: 'Add target zone',
      duplicate: 'Duplicate',
      remove: 'Delete',
    },
    workflow: {
      steps: [
        {
          title: 'Press the activation shortcut',
          body: 'The whip appears near the current cursor and keeps following it without holding the left mouse button.',
        },
        {
          title: 'Crack the whip quickly',
          body:
            'In “Previously selected input” mode, it switches back to the last text field and types there. In “Preset target zones” mode, it sends the zone prompt from the matched zone center.',
        },
        {
          title: 'Auto-click and submit',
          body: 'After a hit, the app clicks the zone center, types the target prompt, and presses Return automatically.',
        },
        {
          title: 'Exit with the hide shortcut or Esc',
          body: 'Leave overlay mode at any time so the whole screen is not stuck in whip follow mode.',
        },
      ],
      noteTitle: 'Coordinate reference',
      noteLines: [
        'x / y represent the top-left corner of a zone',
        'width / height represent the zone size',
        'Every value uses current-screen percentages',
      ],
    },
    footer: {
      restore: 'Restore defaults',
      save: 'Save settings',
      saving: 'Saving...',
      loading: 'Loading...',
    },
    status: {
      shortcutCleared: 'Shortcut cleared. Saving restores the default fallback.',
      defaultsRestored: 'Defaults restored locally. Save to apply them.',
      saved: 'Settings saved and applied immediately.',
      loadFailed: 'Failed to load settings',
      saveFailed: 'Failed to save settings',
    },
  },
}

const LANGUAGE_OPTIONS = Object.freeze([
  { value: LANGUAGE_ZH_CN, label: '中文' },
  { value: LANGUAGE_EN_US, label: 'English' },
])

const MODIFIER_CODES = new Set([
  'MetaLeft',
  'MetaRight',
  'ControlLeft',
  'ControlRight',
  'ShiftLeft',
  'ShiftRight',
  'AltLeft',
  'AltRight',
])

const currentWindow = getCurrentWindow()
const windowLabel = currentWindow.label
const isOverlay = computed(() => windowLabel === 'overlay')
const canvasRef = ref(null)
const loading = ref(false)
const saving = ref(false)
const saveTone = ref('')
const statusKey = ref('')
const statusTextRaw = ref('')
const settings = reactive(createDefaultSettings())

const locale = computed(() => normalizeLanguage(settings.language))
const ui = computed(() => MESSAGES[locale.value])
const summaryChips = computed(() => ui.value.summary.chips)
const languageOptions = computed(() => LANGUAGE_OPTIONS)
const workflowSteps = computed(() => ui.value.workflow.steps)
const statusText = computed(() => {
  if (statusTextRaw.value) {
    return statusTextRaw.value
  }

  if (statusKey.value) {
    return ui.value.status[statusKey.value] ?? ''
  }

  return ''
})

let teardown = null
let unlistenPromptUpdate = null

function normalizeLanguage(value) {
  return value === LANGUAGE_EN_US ? LANGUAGE_EN_US : LANGUAGE_ZH_CN
}

function createId() {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID()
  }

  return `target-${Date.now()}-${Math.random().toString(16).slice(2)}`
}

function defaultTargetName(index) {
  return locale.value === LANGUAGE_EN_US ? `Target ${index}` : `目标 ${index}`
}

function duplicateTargetName(name) {
  return locale.value === LANGUAGE_EN_US ? `${name} Copy` : `${name} 副本`
}

function createTarget(overrides = {}) {
  return {
    id: overrides.id ?? createId(),
    name: overrides.name ?? defaultTargetName(settings.targets.length + 1),
    x: Number.isFinite(Number(overrides.x)) ? Number(overrides.x) : 12,
    y: Number.isFinite(Number(overrides.y)) ? Number(overrides.y) : 18,
    width: Number.isFinite(Number(overrides.width)) ? Number(overrides.width) : 18,
    height: Number.isFinite(Number(overrides.height)) ? Number(overrides.height) : 12,
    promptText: overrides.promptText ?? '',
  }
}

function createDefaultSettings() {
  return {
    promptText: DEFAULT_SETTINGS.promptText,
    showWhipShortcut: DEFAULT_SETTINGS.showWhipShortcut,
    sendPromptShortcut: DEFAULT_SETTINGS.sendPromptShortcut,
    hideWhipShortcut: DEFAULT_SETTINGS.hideWhipShortcut,
    language: DEFAULT_SETTINGS.language,
    crackSendMode: DEFAULT_SETTINGS.crackSendMode,
    targets: [],
  }
}

function clearStatus() {
  statusKey.value = ''
  statusTextRaw.value = ''
  saveTone.value = ''
}

function setLocalizedStatus(key, tone = '') {
  statusKey.value = key
  statusTextRaw.value = ''
  saveTone.value = tone
}

function setRawStatus(message, tone = '') {
  statusKey.value = ''
  statusTextRaw.value = message
  saveTone.value = tone
}

function applyWindowClass() {
  document.body.dataset.windowLabel = windowLabel
}

function assignSettings(target, payload = {}) {
  target.promptText = payload.promptText ?? DEFAULT_SETTINGS.promptText
  target.showWhipShortcut = payload.showWhipShortcut ?? DEFAULT_SETTINGS.showWhipShortcut
  target.sendPromptShortcut = payload.sendPromptShortcut ?? DEFAULT_SETTINGS.sendPromptShortcut
  target.hideWhipShortcut = payload.hideWhipShortcut ?? DEFAULT_SETTINGS.hideWhipShortcut
  target.language = normalizeLanguage(payload.language ?? DEFAULT_SETTINGS.language)
  target.crackSendMode = payload.crackSendMode ?? DEFAULT_SETTINGS.crackSendMode
  target.targets = Array.isArray(payload.targets)
    ? payload.targets.map((item) => createTarget(item))
    : []
}

function syncSettings(payload = {}) {
  assignSettings(settings, payload)
}

function readErrorMessage(error, fallback) {
  if (typeof error === 'string' && error.trim()) {
    return error
  }

  if (error && typeof error === 'object') {
    if (typeof error.message === 'string' && error.message.trim()) {
      return error.message
    }

    const text = error.toString?.()
    if (typeof text === 'string' && text.trim() && text !== '[object Object]') {
      return text
    }
  }

  return fallback
}

function shortcutKeyFromEvent(event) {
  const code = event.code ?? ''

  if (!code || MODIFIER_CODES.has(code)) {
    return ''
  }

  if (code.startsWith('Key')) {
    return code.slice(3).toUpperCase()
  }

  if (code.startsWith('Digit')) {
    return code.slice(5)
  }

  if (code.startsWith('Numpad')) {
    return code
  }

  if (/^F\d+$/.test(code)) {
    return code
  }

  const supportedCodes = new Set([
    'Enter',
    'Tab',
    'Space',
    'Escape',
    'Backspace',
    'Delete',
    'Insert',
    'Home',
    'End',
    'PageUp',
    'PageDown',
    'ArrowUp',
    'ArrowDown',
    'ArrowLeft',
    'ArrowRight',
    'Minus',
    'Equal',
    'BracketLeft',
    'BracketRight',
    'Backslash',
    'Semicolon',
    'Quote',
    'Comma',
    'Period',
    'Slash',
    'Backquote',
  ])

  if (supportedCodes.has(code)) {
    return code
  }

  const key = String(event.key ?? '')
  const keyUpper = key.toUpperCase()
  const shiftedDigits = {
    '!': '1',
    '@': '2',
    '#': '3',
    $: '4',
    '%': '5',
    '^': '6',
    '&': '7',
    '*': '8',
    '(': '9',
    ')': '0',
  }

  if (shiftedDigits[key]) {
    return shiftedDigits[key]
  }

  if (/^[0-9]$/.test(key)) {
    return key
  }

  if (/^[A-Z]$/.test(keyUpper)) {
    return keyUpper
  }

  return ''
}

function formatShortcut(event) {
  const key = shortcutKeyFromEvent(event)

  if (!key) {
    return ''
  }

  const parts = []

  if (event.metaKey || event.ctrlKey) {
    parts.push('CmdOrCtrl')
  }
  if (event.shiftKey) {
    parts.push('Shift')
  }
  if (event.altKey) {
    parts.push('Alt')
  }

  parts.push(key)
  return parts.join('+')
}

function captureShortcut(event, field) {
  if (!field) {
    return
  }

  const modifierOnly = MODIFIER_CODES.has(event.code)
  const noModifier =
    !event.metaKey && !event.ctrlKey && !event.shiftKey && !event.altKey

  if (event.key === 'Escape' && noModifier) {
    settings[field] = ''
    setLocalizedStatus('shortcutCleared')
    return
  }

  if (modifierOnly) {
    return
  }

  const shortcut = formatShortcut(event)
  if (!shortcut) {
    return
  }

  settings[field] = shortcut
  clearStatus()
}

function addTarget() {
  settings.targets.push(
    createTarget({
      name: defaultTargetName(settings.targets.length + 1),
      x: 10 + (settings.targets.length % 3) * 18,
      y: 16 + (settings.targets.length % 2) * 14,
    }),
  )
  clearStatus()
}

function removeTarget(targetId) {
  settings.targets = settings.targets.filter((target) => target.id !== targetId)
  clearStatus()
}

function duplicateTarget(target) {
  settings.targets.push(
    createTarget({
      ...target,
      id: createId(),
      name: duplicateTargetName(target.name),
      x: Math.min(Number(target.x) + 2, 90),
      y: Math.min(Number(target.y) + 2, 90),
    }),
  )
  clearStatus()
}

function targetPayload(target) {
  return {
    id: target.id,
    name: target.name,
    x: Number(target.x),
    y: Number(target.y),
    width: Number(target.width),
    height: Number(target.height),
    promptText: target.promptText,
  }
}

async function loadSettings() {
  loading.value = true
  clearStatus()

  try {
    const result = await invoke('get_prompt_settings')
    syncSettings(result)
  } catch (error) {
    setRawStatus(readErrorMessage(error, ui.value.status.loadFailed), 'error')
  } finally {
    loading.value = false
  }
}

async function saveSettings() {
  saving.value = true
  clearStatus()

  try {
    const result = await invoke('set_prompt_settings', {
      promptText: settings.promptText,
      showWhipShortcut: settings.showWhipShortcut,
      sendPromptShortcut: settings.sendPromptShortcut,
      hideWhipShortcut: settings.hideWhipShortcut,
      language: settings.language,
      crackSendMode: settings.crackSendMode,
      targets: settings.targets.map(targetPayload),
    })
    syncSettings(result)
    setLocalizedStatus('saved', 'success')
  } catch (error) {
    setRawStatus(readErrorMessage(error, ui.value.status.saveFailed), 'error')
  } finally {
    saving.value = false
  }
}

function restoreDefaults() {
  syncSettings({ ...DEFAULT_SETTINGS, language: settings.language })
  setLocalizedStatus('defaultsRestored')
}

onMounted(async () => {
  applyWindowClass()

  if (isOverlay.value) {
    if (!canvasRef.value) {
      return
    }

    teardown = await mountWhipOverlay(canvasRef.value)
    await invoke('overlay_ready')
    return
  }

  await loadSettings()
  unlistenPromptUpdate = await listen('prompt-settings-updated', (event) => {
    syncSettings(event.payload ?? {})
    clearStatus()
  })
})

onBeforeUnmount(() => {
  delete document.body.dataset.windowLabel

  if (teardown) {
    teardown()
    teardown = null
  }

  if (unlistenPromptUpdate) {
    unlistenPromptUpdate()
    unlistenPromptUpdate = null
  }
})
</script>

<template>
  <canvas v-if="isOverlay" ref="canvasRef" class="overlay-canvas"></canvas>

  <main v-else class="settings-shell">
    <section class="settings-panel">
      <header class="settings-hero">
        <div class="settings-hero-copy">
          <div class="settings-brand">
            <img :src="logoMark" class="settings-brand-mark" :alt="ui.brand.logoAlt" />
            <div class="settings-brand-copy">
              <p class="settings-eyebrow">{{ ui.brand.eyebrow }}</p>
              <p class="settings-brand-product">{{ ui.brand.product }}</p>
              <p class="settings-brand-caption">{{ ui.brand.caption }}</p>
            </div>
          </div>

          <h1 class="settings-title">{{ ui.brand.title }}</h1>
          <p class="settings-lead">{{ ui.brand.lead }}</p>
        </div>

        <div class="settings-hero-side">
          <section class="settings-language-panel">
            <div class="settings-card-head">
              <p class="settings-card-kicker">{{ ui.language.label }}</p>
              <p class="settings-card-copy">{{ ui.language.hint }}</p>
            </div>

            <div class="language-switch" role="radiogroup" :aria-label="ui.language.label">
              <button
                v-for="option in languageOptions"
                :key="option.value"
                class="language-switch-option"
                :class="{ 'is-active': settings.language === option.value }"
                type="button"
                role="radio"
                :aria-checked="settings.language === option.value"
                :disabled="loading || saving"
                @click="settings.language = option.value"
              >
                {{ option.label }}
              </button>
            </div>
          </section>

          <div class="settings-summary" :aria-label="ui.summary.aria">
            <span v-for="chip in summaryChips" :key="chip" class="settings-chip">{{ chip }}</span>
          </div>
        </div>
      </header>

      <div class="settings-grid">
        <section class="settings-card settings-card-primary">
          <div class="settings-card-head">
            <p class="settings-card-kicker">{{ ui.cards.fallback.kicker }}</p>
            <h2 class="settings-card-title">{{ ui.cards.fallback.title }}</h2>
            <p class="settings-card-copy">{{ ui.cards.fallback.copy }}</p>
          </div>

          <label class="settings-label" for="prompt-text">{{ ui.fields.promptText }}</label>
          <textarea
            id="prompt-text"
            v-model="settings.promptText"
            class="settings-textarea"
            rows="6"
            :placeholder="ui.placeholders.promptText"
            :disabled="loading || saving"
          />
          <p class="settings-help">{{ ui.help.fallback }}</p>
        </section>

        <section class="settings-card">
          <div class="settings-card-head">
            <p class="settings-card-kicker">{{ ui.cards.shortcuts.kicker }}</p>
            <h2 class="settings-card-title">{{ ui.cards.shortcuts.title }}</h2>
            <p class="settings-card-copy">{{ ui.cards.shortcuts.copy }}</p>
          </div>

          <div class="settings-fields">
            <div class="settings-field">
              <label class="settings-label" for="show-whip-shortcut">{{ ui.fields.showShortcut }}</label>
              <input
                id="show-whip-shortcut"
                v-model="settings.showWhipShortcut"
                class="settings-input settings-input-shortcut"
                type="text"
                readonly
                :placeholder="ui.placeholders.showShortcut"
                :disabled="loading || saving"
                @keydown.prevent="captureShortcut($event, 'showWhipShortcut')"
              />
            </div>

            <div class="settings-field">
              <label class="settings-label" for="send-prompt-shortcut">{{ ui.fields.sendShortcut }}</label>
              <input
                id="send-prompt-shortcut"
                v-model="settings.sendPromptShortcut"
                class="settings-input settings-input-shortcut"
                type="text"
                readonly
                :placeholder="ui.placeholders.sendShortcut"
                :disabled="loading || saving"
                @keydown.prevent="captureShortcut($event, 'sendPromptShortcut')"
              />
            </div>

            <div class="settings-field">
              <label class="settings-label" for="hide-whip-shortcut">{{ ui.fields.hideShortcut }}</label>
              <input
                id="hide-whip-shortcut"
                v-model="settings.hideWhipShortcut"
                class="settings-input settings-input-shortcut"
                type="text"
                readonly
                :placeholder="ui.placeholders.hideShortcut"
                :disabled="loading || saving"
                @keydown.prevent="captureShortcut($event, 'hideWhipShortcut')"
              />
              <p class="settings-help">{{ ui.help.hideShortcut }}</p>
            </div>
          </div>
        </section>

        <section class="settings-card settings-card-wide">
          <div class="settings-card-head">
            <p class="settings-card-kicker">{{ ui.cards.mode.kicker }}</p>
            <h2 class="settings-card-title">{{ ui.cards.mode.title }}</h2>
            <p class="settings-card-copy">{{ ui.cards.mode.copy }}</p>
          </div>

          <div class="mode-switch">
            <label class="mode-option">
              <input v-model="settings.crackSendMode" type="radio" value="selectedInput" />
              <span>{{ ui.modes.selectedInput }}</span>
            </label>
            <label class="mode-option">
              <input v-model="settings.crackSendMode" type="radio" value="targetZones" />
              <span>{{ ui.modes.targetZones }}</span>
            </label>
          </div>
        </section>

        <section class="settings-card settings-card-wide">
          <div class="settings-card-head settings-card-head-row">
            <div>
              <p class="settings-card-kicker">{{ ui.cards.targets.kicker }}</p>
              <h2 class="settings-card-title">{{ ui.cards.targets.title }}</h2>
              <p class="settings-card-copy">{{ ui.cards.targets.copy }}</p>
            </div>

            <button
              class="settings-button settings-button-ghost"
              type="button"
              :disabled="loading || saving"
              @click="addTarget"
            >
              {{ ui.targetActions.add }}
            </button>
          </div>

          <p v-if="settings.crackSendMode !== 'targetZones'" class="settings-help">
            {{ ui.help.targetsInactive }}
          </p>

          <div v-if="settings.targets.length" class="target-list">
            <article v-for="target in settings.targets" :key="target.id" class="target-card">
              <div class="target-card-head">
                <div class="target-main-fields">
                  <div class="settings-field">
                    <label class="settings-label">{{ ui.fields.targetName }}</label>
                    <input
                      v-model="target.name"
                      class="settings-input"
                      type="text"
                      :placeholder="ui.placeholders.targetName"
                      :disabled="loading || saving"
                    />
                  </div>

                  <div class="settings-field">
                    <label class="settings-label">{{ ui.fields.targetPromptText }}</label>
                    <textarea
                      v-model="target.promptText"
                      class="settings-textarea settings-textarea-compact"
                      rows="4"
                      :placeholder="ui.placeholders.targetPromptText"
                      :disabled="loading || saving"
                    />
                  </div>
                </div>

                <div class="target-actions">
                  <button
                    class="settings-button settings-button-muted"
                    type="button"
                    :disabled="loading || saving"
                    @click="duplicateTarget(target)"
                  >
                    {{ ui.targetActions.duplicate }}
                  </button>
                  <button
                    class="settings-button settings-button-danger"
                    type="button"
                    :disabled="loading || saving"
                    @click="removeTarget(target.id)"
                  >
                    {{ ui.targetActions.remove }}
                  </button>
                </div>
              </div>

              <div class="target-grid">
                <div class="settings-field">
                  <label class="settings-label">{{ ui.fields.x }}</label>
                  <input
                    v-model.number="target.x"
                    class="settings-input settings-input-shortcut"
                    type="number"
                    min="0"
                    max="100"
                    step="0.1"
                    :disabled="loading || saving"
                  />
                </div>

                <div class="settings-field">
                  <label class="settings-label">{{ ui.fields.y }}</label>
                  <input
                    v-model.number="target.y"
                    class="settings-input settings-input-shortcut"
                    type="number"
                    min="0"
                    max="100"
                    step="0.1"
                    :disabled="loading || saving"
                  />
                </div>

                <div class="settings-field">
                  <label class="settings-label">{{ ui.fields.width }}</label>
                  <input
                    v-model.number="target.width"
                    class="settings-input settings-input-shortcut"
                    type="number"
                    min="0.1"
                    max="100"
                    step="0.1"
                    :disabled="loading || saving"
                  />
                </div>

                <div class="settings-field">
                  <label class="settings-label">{{ ui.fields.height }}</label>
                  <input
                    v-model.number="target.height"
                    class="settings-input settings-input-shortcut"
                    type="number"
                    min="0.1"
                    max="100"
                    step="0.1"
                    :disabled="loading || saving"
                  />
                </div>
              </div>
            </article>
          </div>

          <div v-else class="settings-empty">
            <p>{{ ui.help.noTargets }}</p>
            <p class="settings-help">{{ ui.help.targetExample }}</p>
          </div>
        </section>

        <section class="settings-card settings-card-wide">
          <div class="settings-card-head">
            <p class="settings-card-kicker">{{ ui.cards.workflow.kicker }}</p>
            <h2 class="settings-card-title">{{ ui.cards.workflow.title }}</h2>
          </div>

          <div class="workflow-layout">
            <ol class="workflow-list">
              <li v-for="(step, index) in workflowSteps" :key="step.title" class="workflow-item">
                <span class="workflow-step">{{ index + 1 }}</span>
                <div>
                  <strong>{{ step.title }}</strong>
                  <p>{{ step.body }}</p>
                </div>
              </li>
            </ol>

            <aside class="settings-note">
              <h3>{{ ui.workflow.noteTitle }}</h3>
              <p v-for="line in ui.workflow.noteLines" :key="line">{{ line }}</p>
              <p class="settings-help">{{ ui.help.coordinateHint }}</p>
            </aside>
          </div>
        </section>

        <section class="settings-card settings-card-wide">
          <div class="settings-card-head">
            <p class="settings-card-kicker">{{ ui.permissions.kicker }}</p>
            <h2 class="settings-card-title">{{ ui.permissions.title }}</h2>
            <p class="settings-card-copy">{{ ui.permissions.copy }}</p>
          </div>

          <div class="settings-help-list">
            <p v-for="item in ui.permissions.items" :key="item" class="settings-help">
              {{ item }}
            </p>
          </div>
        </section>
      </div>

      <footer class="settings-footer">
        <p v-if="statusText" class="settings-status" :data-tone="saveTone">{{ statusText }}</p>
        <div v-else class="settings-status settings-status-placeholder"> </div>

        <div class="settings-actions">
          <button
            class="settings-button settings-button-ghost"
            type="button"
            :disabled="loading || saving"
            @click="restoreDefaults"
          >
            {{ ui.footer.restore }}
          </button>
          <button
            class="settings-button settings-button-primary"
            type="button"
            :disabled="loading || saving"
            @click="saveSettings"
          >
            {{ loading ? ui.footer.loading : saving ? ui.footer.saving : ui.footer.save }}
          </button>
        </div>
      </footer>
    </section>
  </main>
</template>
