import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'

const P = {
  segments: 24,
  segmentLength: 25,
  taper: 0.6,
  gravity: 1.16,
  dropGravity: 0.95,
  damping: 0.962,
  constraintIters: 10,
  maxStretchRatio: 1.2,
  baseTargetAngle: -1.1,
  handleAimByMouseX: 0.44,
  handleAimByMouseY: 0.22,
  handleAimClamp: 2.15,
  handleSpring: 0.72,
  handleAngularDamping: 0.078,
  basePoseSegments: 2,
  basePoseStiffStart: 0.9,
  basePoseStiffEnd: 0.78,
  handleMaxBendDeg: 16,
  tipMaxBendDeg: 132,
  bendRigidityStart: 0.8,
  bendRigidityEnd: 0.12,
  wallBounce: 0.4,
  wallFriction: 0.86,
  crackSpeed: 320,
  crackCooldownMs: 220,
  firstCrackGraceMs: 220,
  lineWidthHandle: 7,
  lineWidthTip: 5,
  outlineWidth: 3,
  handleExtraWidth: 5,
  handleThickSegments: 2,
  bgAlpha: 0,
  arcWidth: 260,
  arcHeight: 180,
  handleMotionSmoothing: 0.24,
  impactDurationMs: 520,
  impactShardCount: 16,
  targetPulseMs: 240,
}

const DEFAULT_SETTINGS = Object.freeze({
  promptText: 'FASTER',
  crackSendMode: 'selectedInput',
  targets: [],
})

const WHIP_CRACK_SOUNDS = [
  '/sounds/A.mp3',
  '/sounds/B.mp3',
  '/sounds/C.mp3',
  '/sounds/D.mp3',
  '/sounds/E.mp3',
]

const clamp = (value, low, high) => Math.max(low, Math.min(high, value))
const lerp = (a, b, t) => a + (b - a) * t

const CRACK_AUDIO_POOL = []

function createCrackAudioPool() {
  if (CRACK_AUDIO_POOL.length || typeof Audio === 'undefined') {
    return
  }

  for (const src of WHIP_CRACK_SOUNDS) {
    const instances = []
    for (let i = 0; i < 3; i += 1) {
      const audio = new Audio(src)
      audio.preload = 'auto'
      instances.push(audio)
    }
    CRACK_AUDIO_POOL.push(instances)
  }
}

function normalizeTarget(target, index) {
  const readNumber = (value, fallback) => {
    const number = Number(value)
    return Number.isFinite(number) ? number : fallback
  }

  return {
    id: target?.id?.trim?.() || `target-${index + 1}`,
    name: target?.name?.trim?.() || `Target ${index + 1}`,
    x: clamp(readNumber(target?.x, 10), 0, 100),
    y: clamp(readNumber(target?.y, 10), 0, 100),
    width: clamp(readNumber(target?.width, 20), 1, 100),
    height: clamp(readNumber(target?.height, 12), 1, 100),
    promptText: target?.promptText ?? '',
  }
}

function targetRect(target, width, height) {
  return {
    x: (target.x / 100) * width,
    y: (target.y / 100) * height,
    width: (target.width / 100) * width,
    height: (target.height / 100) * height,
  }
}

function createImpact(x, y, target) {
  const shards = Array.from({ length: P.impactShardCount }, () => ({
    angle: Math.random() * Math.PI * 2,
    speed: 40 + Math.random() * 80,
    distance: 30 + Math.random() * 120,
    radius: 1.6 + Math.random() * 3.8,
    hueShift: Math.random() * 0.32,
  }))

  return {
    x,
    y,
    bornAt: performance.now(),
    color: target ? '255, 212, 116' : '235, 240, 255',
    targetId: target?.id ?? null,
    shards,
  }
}

function drawRoundedRect(ctx, x, y, width, height, radius) {
  const r = Math.min(radius, width / 2, height / 2)
  ctx.beginPath()
  ctx.moveTo(x + r, y)
  ctx.arcTo(x + width, y, x + width, y + height, r)
  ctx.arcTo(x + width, y + height, x, y + height, r)
  ctx.arcTo(x, y + height, x, y, r)
  ctx.arcTo(x, y, x + width, y, r)
  ctx.closePath()
}

function wrapPi(angle) {
  let value = angle
  while (value > Math.PI) value -= Math.PI * 2
  while (value < -Math.PI) value += Math.PI * 2
  return value
}

export async function mountWhipOverlay(canvas) {
  const ctx = canvas.getContext('2d')
  createCrackAudioPool()

  let W = 0
  let H = 0
  let mouseX = null
  let mouseY = null
  let prevMouseX = null
  let prevMouseY = null
  let whip = null
  let tracking = false
  let pinned = false
  let dropping = false
  let dismissAfterDrop = false
  let lastCrackTime = 0
  let whipSpawnTime = 0
  let handleAngle = P.baseTargetAngle
  let handleAngVel = 0
  let frameId = 0
  let smoothedMouseDX = 0
  let smoothedMouseDY = 0
  let unlistenSpawn = null
  let unlistenDrop = null
  let unlistenSettings = null
  let unlistenHidden = null
  const targetPulseUntil = new Map()
  const impacts = []
  const overlaySettings = {
    promptText: DEFAULT_SETTINGS.promptText,
    crackSendMode: DEFAULT_SETTINGS.crackSendMode,
    targets: [],
  }

  function setOverlayInteraction(passthrough) {
    document.body.dataset.overlayMode = passthrough ? 'passthrough' : 'interactive'
    invoke('set_overlay_passthrough', { passthrough }).catch(() => {})
  }

  function applyOverlaySettings(payload = {}) {
    overlaySettings.promptText = payload.promptText ?? DEFAULT_SETTINGS.promptText
    overlaySettings.crackSendMode = payload.crackSendMode ?? DEFAULT_SETTINGS.crackSendMode
    overlaySettings.targets = Array.isArray(payload.targets)
      ? payload.targets.map(normalizeTarget)
      : []
  }

  async function loadOverlaySettings() {
    try {
      const result = await invoke('get_prompt_settings')
      applyOverlaySettings(result)
    } catch {
      applyOverlaySettings(DEFAULT_SETTINGS)
    }
  }

  function resize() {
    W = canvas.width = window.innerWidth
    H = canvas.height = window.innerHeight

    if (mouseX == null || mouseY == null) {
      mouseX = W / 2
      mouseY = H / 2
      prevMouseX = mouseX
      prevMouseY = mouseY
    }

    draw(performance.now())
  }

  function ensureLoop() {
    if (!frameId) {
      frameId = window.requestAnimationFrame(loop)
    }
  }

  function clearLoop() {
    if (frameId) {
      window.cancelAnimationFrame(frameId)
      frameId = 0
    }
  }

  function playCrackSound() {
    createCrackAudioPool()
    if (!CRACK_AUDIO_POOL.length) return

    const pool = CRACK_AUDIO_POOL[Math.floor(Math.random() * CRACK_AUDIO_POOL.length)]
    const audio = pool.find((instance) => instance.paused || instance.ended) ?? pool[0]
    audio.currentTime = 0
    audio.play().catch(() => {})
  }

  function distancePointToSegment(px, py, ax, ay, bx, by) {
    const dx = bx - ax
    const dy = by - ay

    if (dx === 0 && dy === 0) {
      return Math.hypot(px - ax, py - ay)
    }

    const t = clamp(((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy), 0, 1)
    const sx = ax + dx * t
    const sy = ay + dy * t
    return Math.hypot(px - sx, py - sy)
  }

  function moveWhipHandleTo(x, y) {
    if (!whip) {
      return
    }

    const dx = x - whip[0].x
    const dy = y - whip[0].y

    for (const point of whip) {
      point.x += dx
      point.y += dy
      point.px += dx
      point.py += dy
    }

    if (whip.length >= 2) {
      const next = whip[1]
      handleAngle = wrapPi(Math.atan2(next.y - whip[0].y, next.x - whip[0].x))
    } else {
      handleAngle = P.baseTargetAngle
    }
    handleAngVel = 0
  }

  function setTrackingPointer(x, y) {
    mouseX = clamp(x, 0, W)
    mouseY = clamp(y, 0, H)
    if (prevMouseX == null || prevMouseY == null) {
      prevMouseX = mouseX
      prevMouseY = mouseY
    }
  }

  function spawnWhip(mx, my) {
    tracking = true
    pinned = false
    dropping = false
    dismissAfterDrop = false
    lastCrackTime = 0
    whipSpawnTime = Date.now()
    smoothedMouseDX = 0
    smoothedMouseDY = 0
    const points = []

    for (let i = 0; i < P.segments; i += 1) {
      const t = i / (P.segments - 1)
      const x = mx + t * P.arcWidth
      const y = my - Math.sin(t * Math.PI * 0.75) * P.arcHeight
      points.push({ x, y, px: x, py: y })
    }

    return points
  }

  function segLen(i) {
    const t = i / (P.segments - 1)
    return P.segmentLength * (1 - t * (1 - P.taper))
  }

  function catmullPoint(points, i) {
    const n = points.length
    if (n === 0) return { x: 0, y: 0 }
    if (i < 0) {
      if (n >= 2) {
        return {
          x: 2 * points[0].x - points[1].x,
          y: 2 * points[0].y - points[1].y,
        }
      }
      return { x: points[0].x, y: points[0].y }
    }
    if (i >= n) {
      if (n >= 2) {
        const a = points[n - 2]
        const b = points[n - 1]
        return { x: 2 * b.x - a.x, y: 2 * b.y - a.y }
      }
      return { x: points[n - 1].x, y: points[n - 1].y }
    }
    return points[i]
  }

  function whipSegmentBezier(points, i) {
    const p0 = catmullPoint(points, i - 1)
    const p1 = points[i]
    const p2 = points[i + 1]
    const p3 = catmullPoint(points, i + 2)

    return {
      cp1x: p1.x + (p2.x - p0.x) / 6,
      cp1y: p1.y + (p2.y - p0.y) / 6,
      cp2x: p2.x - (p3.x - p1.x) / 6,
      cp2y: p2.y - (p3.y - p1.y) / 6,
      x2: p2.x,
      y2: p2.y,
    }
  }

  function resolveTargetAt(x, y) {
    if (
      overlaySettings.crackSendMode !== 'targetZones' ||
      !overlaySettings.targets.length ||
      W <= 0 ||
      H <= 0
    ) {
      return null
    }

    const px = clamp((x / W) * 100, 0, 100)
    const py = clamp((y / H) * 100, 0, 100)
    return (
      overlaySettings.targets.find(
        (target) =>
          px >= target.x &&
          px <= target.x + target.width &&
          py >= target.y &&
          py <= target.y + target.height,
      ) ?? null
    )
  }

  function spawnImpact(x, y, target) {
    impacts.push(createImpact(x, y, target))
    if (target?.id) {
      targetPulseUntil.set(target.id, performance.now() + P.targetPulseMs)
    }
    ensureLoop()
  }

  function requestHideOverlay(animated = false) {
    tracking = false
    pinned = false
    setOverlayInteraction(false)

    if (animated && whip && !dropping) {
      dropping = true
      dismissAfterDrop = true
      ensureLoop()
      return
    }

    whip = null
    impacts.length = 0
    clearLoop()
    draw(performance.now())
    invoke('hide_overlay').catch(() => {})
  }

  function onMouseMove(event) {
    setTrackingPointer(event.clientX, event.clientY)
    if (tracking) {
      ensureLoop()
    }
  }

  function onContextMenu(event) {
    event.preventDefault()

    if (!whip || dropping) {
      return
    }

    setTrackingPointer(event.clientX, event.clientY)
    moveWhipHandleTo(mouseX, mouseY)
    pinned = !pinned
    tracking = !pinned
    whipSpawnTime = Date.now()
    lastCrackTime = 0
    smoothedMouseDX = 0
    smoothedMouseDY = 0

    if (tracking) {
      ensureLoop()
    } else {
      draw(performance.now())
    }
  }

  function onKeyDown(event) {
    if (event.key !== 'Escape') {
      return
    }

    event.preventDefault()
    requestHideOverlay(true)
  }

  function updateHandleAim() {
    if (dropping || pinned || !tracking) return

    const rawDX = (mouseX ?? W / 2) - (prevMouseX ?? mouseX ?? W / 2)
    const rawDY = (mouseY ?? H / 2) - (prevMouseY ?? mouseY ?? H / 2)
    smoothedMouseDX = lerp(smoothedMouseDX, rawDX, P.handleMotionSmoothing)
    smoothedMouseDY = lerp(smoothedMouseDY, rawDY, P.handleMotionSmoothing)
    const delta = clamp(
      smoothedMouseDX * P.handleAimByMouseX + smoothedMouseDY * P.handleAimByMouseY,
      -P.handleAimClamp,
      P.handleAimClamp,
    )
    const target = P.baseTargetAngle + delta
    const err = wrapPi(target - handleAngle)
    handleAngVel += err * P.handleSpring
    handleAngVel *= P.handleAngularDamping
    handleAngle = wrapPi(handleAngle + handleAngVel)
  }

  function applyBasePose() {
    if (!whip || dropping || pinned || !tracking) return

    const dx = Math.cos(handleAngle)
    const dy = Math.sin(handleAngle)
    const guided = Math.min(P.basePoseSegments, whip.length - 1)

    for (let i = 1; i <= guided; i += 1) {
      const t = (i - 1) / Math.max(guided - 1, 1)
      const stiff = lerp(P.basePoseStiffStart, P.basePoseStiffEnd, t)
      const prev = whip[i - 1]
      const point = whip[i]
      const targetLen = segLen(i - 1)
      const tx = prev.x + dx * targetLen
      const ty = prev.y + dy * targetLen
      point.x = lerp(point.x, tx, stiff)
      point.y = lerp(point.y, ty, stiff)
    }
  }

  function applyBendLimits() {
    if (!whip || whip.length < 3) return

    for (let i = 1; i < whip.length - 1; i += 1) {
      const a = whip[i - 1]
      const b = whip[i]
      const c = whip[i + 1]

      const v1x = a.x - b.x
      const v1y = a.y - b.y
      const v2x = c.x - b.x
      const v2y = c.y - b.y
      const l1 = Math.hypot(v1x, v1y) || 0.0001
      const l2 = Math.hypot(v2x, v2y) || 0.0001
      const n1x = v1x / l1
      const n1y = v1y / l1
      const n2x = v2x / l2
      const n2y = v2y / l2

      const dot = clamp(n1x * n2x + n1y * n2y, -1, 1)
      const angle = Math.acos(dot)
      const t = i / (whip.length - 2)
      const maxBend = (lerp(P.handleMaxBendDeg, P.tipMaxBendDeg, t) * Math.PI) / 180
      const bend = Math.PI - angle

      if (bend <= maxBend) continue

      const cross = n1x * n2y - n1y * n2x
      const sign = cross >= 0 ? 1 : -1
      const targetAngle = Math.PI - maxBend
      const targetA = Math.atan2(n1y, n1x) + sign * targetAngle
      const tx = b.x + Math.cos(targetA) * l2
      const ty = b.y + Math.sin(targetA) * l2
      const rigidity = lerp(P.bendRigidityStart, P.bendRigidityEnd, t)

      c.x = lerp(c.x, tx, rigidity)
      c.y = lerp(c.y, ty, rigidity)
    }
  }

  function capSegmentStretch() {
    if (!whip || whip.length < 2) return

    for (let i = 0; i < whip.length - 1; i += 1) {
      const a = whip[i]
      const b = whip[i + 1]
      const dx = b.x - a.x
      const dy = b.y - a.y
      const dist = Math.hypot(dx, dy) || 0.0001
      const maxLen = segLen(i) * P.maxStretchRatio

      if (dist <= maxLen) continue

      const k = maxLen / dist
      b.x = a.x + dx * k
      b.y = a.y + dy * k
    }
  }

  function applyWallCollisions() {
    if (!whip || dropping) return

    for (let i = 1; i < whip.length; i += 1) {
      const point = whip[i]
      let vx = point.x - point.px
      let vy = point.y - point.py
      let hit = false

      if (point.x < 0) {
        point.x = 0
        if (vx < 0) vx = -vx * P.wallBounce
        vy *= P.wallFriction
        hit = true
      } else if (point.x > W) {
        point.x = W
        if (vx > 0) vx = -vx * P.wallBounce
        vy *= P.wallFriction
        hit = true
      }

      if (point.y < 0) {
        point.y = 0
        if (vy < 0) vy = -vy * P.wallBounce
        vx *= P.wallFriction
        hit = true
      } else if (point.y > H) {
        point.y = H
        if (vy > 0) vy = -vy * P.wallBounce
        vx *= P.wallFriction
        hit = true
      }

      if (hit) {
        point.px = point.x - vx
        point.py = point.y - vy
      }
    }
  }

  function updateImpacts(now) {
    for (let i = impacts.length - 1; i >= 0; i -= 1) {
      if (now - impacts[i].bornAt > P.impactDurationMs) {
        impacts.splice(i, 1)
      }
    }

    for (const [targetId, until] of targetPulseUntil.entries()) {
      if (until <= now) {
        targetPulseUntil.delete(targetId)
      }
    }
  }

  function update(now) {
    updateImpacts(now)

    if (!whip) {
      prevMouseX = mouseX
      prevMouseY = mouseY
      return
    }

    if (pinned) {
      prevMouseX = mouseX
      prevMouseY = mouseY
      smoothedMouseDX = 0
      smoothedMouseDY = 0
      return
    }

    const gravity = dropping ? P.dropGravity : P.gravity
    updateHandleAim()

    const start = dropping ? 0 : 1
    for (let i = start; i < whip.length; i += 1) {
      const point = whip[i]
      const vx = (point.x - point.px) * P.damping
      const vy = (point.y - point.py) * P.damping
      point.px = point.x
      point.py = point.y
      point.x += vx
      point.y += vy + gravity
    }

    if (!dropping && tracking) {
      const handleX = mouseX ?? W / 2
      const handleY = mouseY ?? H / 2
      whip[0].x = handleX
      whip[0].y = handleY
      whip[0].px = handleX
      whip[0].py = handleY
    }

    capSegmentStretch()
    applyWallCollisions()
    applyBasePose()

    for (let iter = 0; iter < P.constraintIters; iter += 1) {
      for (let i = 0; i < whip.length - 1; i += 1) {
        const a = whip[i]
        const b = whip[i + 1]
        const dx = b.x - a.x
        const dy = b.y - a.y
        const dist = Math.sqrt(dx * dx + dy * dy) || 0.0001
        const target = segLen(i)
        const diff = ((dist - target) / dist) * 0.5
        const ox = dx * diff
        const oy = dy * diff

        if (i === 0 && !dropping) {
          b.x -= ox * 2
          b.y -= oy * 2
        } else {
          a.x += ox
          a.y += oy
          b.x -= ox
          b.y -= oy
        }
      }

      applyBendLimits()
      if (!dropping) applyBasePose()
      capSegmentStretch()
      applyWallCollisions()
    }

    const tip = whip[whip.length - 1]
    const tipVel = Math.hypot(tip.x - tip.px, tip.y - tip.py)

    if (!dropping && tracking && tipVel > P.crackSpeed) {
      const currentTime = Date.now()
      if (
        currentTime - whipSpawnTime >= P.firstCrackGraceMs &&
        currentTime - lastCrackTime > P.crackCooldownMs
      ) {
        lastCrackTime = currentTime
        const crackX = clamp(tip.x, 0, W)
        const crackY = clamp(tip.y, 0, H)
        const hitTarget = resolveTargetAt(crackX, crackY)
        playCrackSound()
        spawnImpact(crackX, crackY, hitTarget)
        invoke('whip_crack', {
          payload: {
            x: crackX,
            y: crackY,
          },
        }).catch(() => {})
      }
    }

    if (dropping && whip.every((point) => point.y > H + 80)) {
      dropping = false
      whip = null

      if (dismissAfterDrop) {
        dismissAfterDrop = false
        clearLoop()
        draw(now)
        invoke('hide_overlay').catch(() => {})
        return
      }
    }

    prevMouseX = mouseX
    prevMouseY = mouseY
  }

  function drawTargets(now) {
    if (
      overlaySettings.crackSendMode !== 'targetZones' ||
      !overlaySettings.targets.length
    ) {
      return
    }

    ctx.save()
    ctx.lineWidth = 1.5
    ctx.setLineDash([12, 10])
    ctx.font = "12px 'SF Mono', Menlo, Monaco, Consolas, monospace"
    ctx.textBaseline = 'middle'

    for (const target of overlaySettings.targets) {
      const rect = targetRect(target, W, H)
      const pulse = clamp(((targetPulseUntil.get(target.id) ?? 0) - now) / P.targetPulseMs, 0, 1)
      const baseFill = 0.05 + pulse * 0.18
      const baseStroke = 0.24 + pulse * 0.5

      drawRoundedRect(ctx, rect.x, rect.y, rect.width, rect.height, 18)
      ctx.fillStyle = `rgba(255, 204, 110, ${baseFill})`
      ctx.strokeStyle = `rgba(255, 214, 126, ${baseStroke})`
      ctx.fill()
      ctx.stroke()

      const labelWidth = Math.max(84, ctx.measureText(target.name).width + 28)
      const labelX = clamp(rect.x + 10, 8, Math.max(8, W - labelWidth - 8))
      const labelY = Math.max(10, rect.y - 16)

      drawRoundedRect(ctx, labelX, labelY, labelWidth, 24, 12)
      ctx.setLineDash([])
      ctx.fillStyle = `rgba(18, 14, 10, ${0.68 + pulse * 0.2})`
      ctx.fill()
      ctx.fillStyle = `rgba(255, 236, 207, ${0.92})`
      ctx.fillText(target.name, labelX + 14, labelY + 12)
      ctx.setLineDash([12, 10])
    }

    ctx.restore()
  }

  function drawWhip() {
    if (!whip) {
      return
    }

    ctx.lineCap = 'round'
    ctx.lineJoin = 'round'
    ctx.strokeStyle = '#fff'

    if (whip.length >= 2) {
      ctx.beginPath()
      ctx.moveTo(whip[0].x, whip[0].y)
      for (let i = 0; i < whip.length - 1; i += 1) {
        const { cp1x, cp1y, cp2x, cp2y, x2, y2 } = whipSegmentBezier(whip, i)
        ctx.bezierCurveTo(cp1x, cp1y, cp2x, cp2y, x2, y2)
      }
      ctx.lineWidth = P.lineWidthTip + P.outlineWidth * 2
      ctx.stroke()

      const thickLinks = Math.min(P.handleThickSegments, whip.length - 1)
      if (thickLinks > 0 && P.handleExtraWidth > 0) {
        ctx.beginPath()
        ctx.moveTo(whip[0].x, whip[0].y)
        for (let i = 0; i < thickLinks; i += 1) {
          const { cp1x, cp1y, cp2x, cp2y, x2, y2 } = whipSegmentBezier(whip, i)
          ctx.bezierCurveTo(cp1x, cp1y, cp2x, cp2y, x2, y2)
        }
        ctx.lineWidth = P.lineWidthHandle + P.handleExtraWidth + P.outlineWidth * 2
        ctx.stroke()
      }
    }

    ctx.strokeStyle = '#111'
    for (let i = 0; i < whip.length - 1; i += 1) {
      const t = i / Math.max(1, whip.length - 2)
      const extra = i < P.handleThickSegments ? P.handleExtraWidth : 0
      ctx.lineWidth = lerp(P.lineWidthHandle, P.lineWidthTip, t) + extra
      const { cp1x, cp1y, cp2x, cp2y, x2, y2 } = whipSegmentBezier(whip, i)
      ctx.beginPath()
      ctx.moveTo(whip[i].x, whip[i].y)
      ctx.bezierCurveTo(cp1x, cp1y, cp2x, cp2y, x2, y2)
      ctx.stroke()
    }
  }

  function drawCursorAura() {
    if (!tracking || mouseX == null || mouseY == null) {
      return
    }

    ctx.save()
    const gradient = ctx.createRadialGradient(mouseX, mouseY, 0, mouseX, mouseY, 34)
    gradient.addColorStop(0, 'rgba(255, 220, 160, 0.14)')
    gradient.addColorStop(1, 'rgba(255, 220, 160, 0)')
    ctx.fillStyle = gradient
    ctx.beginPath()
    ctx.arc(mouseX, mouseY, 34, 0, Math.PI * 2)
    ctx.fill()
    ctx.restore()
  }

  function drawImpacts(now) {
    if (!impacts.length) {
      return
    }

    ctx.save()
    ctx.globalCompositeOperation = 'lighter'

    for (const effect of impacts) {
      const progress = clamp((now - effect.bornAt) / P.impactDurationMs, 0, 1)
      const alpha = 1 - progress
      const ringRadius = lerp(18, 116, progress)

      ctx.strokeStyle = `rgba(${effect.color}, ${0.74 * alpha})`
      ctx.lineWidth = lerp(10, 1.2, progress)
      ctx.beginPath()
      ctx.arc(effect.x, effect.y, ringRadius, 0, Math.PI * 2)
      ctx.stroke()

      ctx.fillStyle = `rgba(${effect.color}, ${0.22 * alpha})`
      ctx.beginPath()
      ctx.arc(effect.x, effect.y, lerp(10, 42, progress), 0, Math.PI * 2)
      ctx.fill()

      for (const shard of effect.shards) {
        const distance = shard.distance * progress
        const x = effect.x + Math.cos(shard.angle) * distance
        const y = effect.y + Math.sin(shard.angle) * distance
        ctx.fillStyle = `rgba(${effect.color}, ${0.9 * alpha})`
        ctx.beginPath()
        ctx.arc(x, y, shard.radius * (1 - progress * 0.68), 0, Math.PI * 2)
        ctx.fill()
      }
    }

    ctx.restore()
  }

  function draw(now) {
    ctx.clearRect(0, 0, W, H)

    if (P.bgAlpha > 0) {
      ctx.fillStyle = `rgba(0,0,0,${P.bgAlpha})`
      ctx.fillRect(0, 0, W, H)
    }

    drawTargets(now)
    drawCursorAura()
    drawWhip()
    drawImpacts(now)
  }

  function loop(now = performance.now()) {
    update(now)
    draw(now)

    if (whip || impacts.length) {
      const keepRunning = tracking || dropping || impacts.length > 0
      if (keepRunning) {
        frameId = window.requestAnimationFrame(loop)
        return
      }
    }

    frameId = 0
  }

  resize()
  setOverlayInteraction(false)
  await loadOverlaySettings()
  window.addEventListener('resize', resize)
  document.addEventListener('mousemove', onMouseMove)
  document.addEventListener('contextmenu', onContextMenu)
  document.addEventListener('keydown', onKeyDown)

  unlistenSpawn = await listen('spawn-whip', (event) => {
    const x = clamp(event.payload?.x ?? W / 2, 0, W)
    const y = clamp(event.payload?.y ?? H / 2, 0, H)
    setTrackingPointer(x, y)
    whip = spawnWhip(x, y)
    setOverlayInteraction(false)
    ensureLoop()
  })

  unlistenDrop = await listen('drop-whip', () => {
    if (!whip || dropping) {
      requestHideOverlay(false)
      return
    }

    tracking = false
    pinned = false
    dropping = true
    dismissAfterDrop = true
    ensureLoop()
  })

  unlistenSettings = await listen('prompt-settings-updated', (event) => {
    applyOverlaySettings(event.payload ?? DEFAULT_SETTINGS)
    draw(performance.now())
  })

  unlistenHidden = await listen('overlay-hidden', () => {
    tracking = false
    pinned = false
    dropping = false
    dismissAfterDrop = false
    whip = null
    impacts.length = 0
    clearLoop()
    draw(performance.now())
  })

  return () => {
    clearLoop()
    window.removeEventListener('resize', resize)
    document.removeEventListener('mousemove', onMouseMove)
    document.removeEventListener('contextmenu', onContextMenu)
    document.removeEventListener('keydown', onKeyDown)
    unlistenSpawn?.()
    unlistenDrop?.()
    unlistenSettings?.()
    unlistenHidden?.()
    delete document.body.dataset.overlayMode
  }
}
