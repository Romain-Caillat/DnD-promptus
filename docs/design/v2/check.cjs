// Renders every v2 board in Chromium, reports page errors and horizontal
// overflow at phone width, and saves a screenshot of each screen.
// Usage: node docs/design/v2/check.cjs [outDir]
// Needs Playwright (global install, or NODE_PATH pointing at it).
const path = require('path')
const fs = require('fs')
const { chromium } = require('playwright')

const dir = __dirname
const out = process.argv[2] || path.join(require('os').tmpdir(), 'promptus-v2-shots')
fs.mkdirSync(out, { recursive: true })

;(async () => {
  const browser = await chromium.launch()
  let failed = false
  for (const file of fs.readdirSync(dir).filter((f) => f.endsWith('.html'))) {
    const page = await browser.newPage({ viewport: { width: 1320, height: 900 } })
    const errors = []
    page.on('pageerror', (e) => errors.push(e.message))
    page.on('console', (m) => m.type() === 'error' && errors.push(m.text()))
    await page.goto('file://' + path.join(dir, file))
    await page.waitForTimeout(1500)
    await page.locator('#wrap').screenshot({ path: path.join(out, file.replace('.html', '.png')) })
    await page.setViewportSize({ width: 390, height: 800 })
    await page.waitForTimeout(300)
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth > innerWidth)
    if (errors.length || overflow) failed = true
    console.log(file, errors.length ? 'ERRORS ' + errors.join(' | ') : 'ok', overflow ? 'OVERFLOW at 390px' : '')
    await page.close()
  }
  await browser.close()
  console.log('screenshots in', out)
  process.exit(failed ? 1 : 0)
})()
