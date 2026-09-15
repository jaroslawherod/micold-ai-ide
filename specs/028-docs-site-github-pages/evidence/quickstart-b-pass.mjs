import { chromium, devices } from 'playwright';
import { writeFile } from 'node:fs/promises';

const BASE = 'https://jaroslawherod.github.io/micold-ai-ide/';
const out = {};
const browser = await chromium.launch();
const shot = (page, name, opts = {}) => page.screenshot({ path: `shots/${name}.png`, ...opts });

// ---- B1: cold, laptop viewport, no scrolling
{
  const ctx = await browser.newContext({ viewport: { width: 1366, height: 768 } });
  const page = await ctx.newPage();
  const t0 = Date.now();
  await page.goto(BASE, { waitUntil: 'load' });
  const loaded = Date.now() - t0;
  await shot(page, 'b1-home-1366x768');
  const fold = await page.evaluate(() => {
    const vh = innerHeight;
    const visible = el => { const r = el.getBoundingClientRect(); return r.bottom > 0 && r.top < vh && r.width > 0 && r.height > 0; };
    const text = [...document.querySelectorAll('main h1, main h2, main p, main li')].filter(visible).map(e => e.innerText.trim()).filter(Boolean);
    const install = [...document.querySelectorAll('main a')].filter(a => /install/i.test(a.getAttribute('href') || '') && visible(a))
      .map(a => ({ text: a.innerText.trim(), href: a.getAttribute('href'), top: Math.round(a.getBoundingClientRect().top) }));
    const imgs = [...document.querySelectorAll('main img, main video')].filter(visible).map(e => e.getAttribute('alt') || e.getAttribute('aria-label'));
    return { text, install, imgs, scrollY };
  });
  const t1 = Date.now();
  await page.locator('main a[href*="install"]').first().click();
  await page.waitForLoadState('load');
  out.b1 = { loadMs: loaded, fold, installReachedMs: Date.now() - t1, installUrl: page.url(), installH1: await page.locator('main h1').first().innerText() };
  await shot(page, 'b1-install-1366x768');
  await ctx.close();
}

// ---- B2: find a topic by search, from a page that is not the answer
const topics = [
  { topic: 'revealing hidden agent worktrees', from: 'user-guide/settings.html', query: 'agent worktrees', expect: /worktrees-and-sessions/, heading: 'Agent worktrees' },
  { topic: 'the scrollback limit', from: 'user-guide/project-selection.html', query: 'scrollback', expect: /worktrees-and-sessions|daemon/, heading: /scrollback/i },
  { topic: 'running the service in a container', from: 'user-guide/help-about.html', query: 'container', expect: /sandboxed-daemon/, heading: 'Running the session service in a container' },
  { topic: 'opening About (random)', from: 'user-guide/icons.html', query: 'about', expect: /help-about/, heading: 'Opening About' },
  { topic: 'choosing your theme (random)', from: 'install.html', query: 'theme', expect: /appearance-theming/, heading: 'Choosing your theme' },
  { topic: "when a branch can't be used (random)", from: 'daemon.html', query: 'branch', expect: /worktrees-and-sessions/, heading: /branch can.t be used/i },
];
out.b2 = [];
{
  const ctx = await browser.newContext({ viewport: { width: 1366, height: 768 } });
  const page = await ctx.newPage();
  for (const t of topics) {
    await page.goto(BASE + t.from, { waitUntil: 'load' });
    const t0 = Date.now();
    const bar = page.locator('#mdbook-searchbar');
    const openedAlready = await bar.isVisible();
    if (!openedAlready) await page.locator('#mdbook-search-toggle').click();
    await bar.click();
    await bar.fill('');
    await bar.pressSequentially(t.query, { delay: 60 });
    const results = page.locator('#mdbook-searchresults li a[href]');
    try {
      await page.waitForFunction(() => document.querySelectorAll('#mdbook-searchresults li a[href]').length > 0, undefined, { timeout: 10000 });
    } catch {
      await shot(page, `b2-${out.b2.length + 1}-noresults`);
      out.b2.push({ ...t, expect: String(t.expect), heading: String(t.heading), openedAlready, url: page.url(), barValue: await bar.inputValue(), noResults: true });
      continue;
    }
    const top = await results.evaluateAll(as => as.slice(0, 5).map(a => ({ text: a.innerText.replace(/\s+/g, ' ').trim().slice(0, 90), href: a.getAttribute('href') })));
    // A reader clicks the first result whose page and heading match what they are after.
    let pick = top.findIndex(r => t.expect.test(r.href) && (typeof t.heading === 'string' ? r.text.includes(t.heading) : t.heading.test(r.text)));
    if (pick < 0) pick = top.findIndex(r => t.expect.test(r.href));
    let landed = null, headingVisible = false;
    if (pick >= 0) {
      await results.nth(pick).click();
      await page.waitForLoadState('load');
      landed = page.url();
      const h = page.locator('main h1, main h2, main h3', { hasText: t.heading }).first();
      headingVisible = await h.isVisible().catch(() => false);
    }
    const ms = Date.now() - t0;
    await shot(page, `b2-${out.b2.length + 1}`);
    out.b2.push({ ...t, expect: String(t.expect), heading: String(t.heading), top, pickedRank: pick + 1, landed, headingVisible, ms });
  }
  // Navigation route: the sidebar from the home page, one click per page.
  await page.goto(BASE, { waitUntil: 'load' });
  out.b2nav = await page.evaluate(() => [...document.querySelectorAll('#mdbook-sidebar a, nav a')].map(a => a.innerText.trim()).filter(Boolean).slice(0, 40));
  await ctx.close();
}

// ---- B3: one design language, both schemes
out.b3 = {};
for (const scheme of ['light', 'dark']) {
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 }, colorScheme: scheme });
  const page = await ctx.newPage();
  await page.goto(BASE + 'user-guide/appearance-theming.html', { waitUntil: 'load' });
  await shot(page, `b3-${scheme}-top`);
  const img = page.locator('main img').first();
  await img.scrollIntoViewIfNeeded();
  await page.waitForTimeout(800);
  await shot(page, `b3-${scheme}-image`);
  out.b3[scheme] = await page.evaluate(() => {
    const cs = s => { const e = document.querySelector(s); if (!e) return null; const c = getComputedStyle(e); return { bg: c.backgroundColor, border: c.borderStyle + ' ' + c.borderWidth + ' ' + c.borderColor, shadow: c.boxShadow, radius: c.borderRadius, font: c.fontFamily }; };
    return { html: document.documentElement.className, header: cs('#mdbook-menu-bar') || cs('.menu-bar'), sidebar: cs('#mdbook-sidebar') || cs('.sidebar'), body: cs('body'), img: cs('main img'), pre: cs('main pre') };
  });
  await ctx.close();
}

// ---- B4: motion
{
  const ctx = await browser.newContext({ viewport: { width: 1366, height: 900 } });
  const page = await ctx.newPage();
  const media = [];
  page.on('request', r => { if (/\.(webm|mp4|mov|gif)(\?|$)/i.test(r.url())) media.push({ url: r.url(), at: Date.now() }); });
  await page.goto(BASE + 'user-guide/worktrees-and-sessions.html', { waitUntil: 'load' });
  const v = page.locator('main video').first();
  await v.scrollIntoViewIfNeeded();
  await page.waitForTimeout(1500);
  const a = await v.screenshot({ path: 'shots/b4-idle-1.png' });
  await page.waitForTimeout(3000);
  const b = await v.screenshot({ path: 'shots/b4-idle-2.png' });
  const idle = await page.evaluate(() => [...document.querySelectorAll('main video')].map(v => ({ paused: v.paused, t: v.currentTime, autoplay: v.autoplay, preload: v.preload, poster: v.getAttribute('poster'), sources: [...v.querySelectorAll('source')].map(s => s.getAttribute('src')) })));
  const idleMediaRequests = media.length;
  const tPlay = Date.now();
  await v.evaluate(v => v.play());
  await page.waitForTimeout(4000);
  await v.screenshot({ path: 'shots/b4-playing.png' });
  const playing = await v.evaluate(v => ({ paused: v.paused, t: v.currentTime, duration: v.duration, loop: v.loop, muted: v.muted, audioBytes: v.webkitAudioDecodedByteCount, src: v.currentSrc }));
  out.b4 = { idle, idleFramesIdentical: a.equals(b), idleMediaRequests, mediaAfterPlay: media.filter(m => m.at >= tPlay).map(m => m.url), playing };
  // transitions, with and without reduced motion
  const probe = () => page.evaluate(() => {
    const els = [...document.querySelectorAll('a, button, .chapter li, #mdbook-sidebar, body')].slice(0, 300);
    const durs = new Set(); for (const e of els) { const c = getComputedStyle(e); if (c.transitionDuration && c.transitionDuration !== '0s') durs.add(c.transitionDuration + ' ' + c.transitionTimingFunction); }
    const vids = [...document.querySelectorAll('main video')].map(v => v.paused);
    return { nonZeroTransitions: [...durs].slice(0, 8), videosPaused: vids };
  });
  out.b4.transitionsDefault = await probe();
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.reload({ waitUntil: 'load' });
  await page.waitForTimeout(2000);
  out.b4.transitionsReduced = await probe();
  await ctx.close();
}

// ---- B5: phone (emulated)
out.b5 = [];
for (const dev of ['iPhone 13', 'Pixel 7']) {
  const ctx = await browser.newContext({ ...devices[dev] });
  const page = await ctx.newPage();
  for (const p of ['user-guide/worktrees-and-sessions.html', 'user-guide/appearance-theming.html']) {
    await page.goto(BASE + p, { waitUntil: 'load' });
    await page.evaluate(async () => { for (let y = 0; y < document.body.scrollHeight; y += 600) { scrollTo(0, y); await new Promise(r => setTimeout(r, 60)); } scrollTo(0, 0); });
    await page.waitForTimeout(800);
    const m = await page.evaluate(() => {
      const vw = document.documentElement.clientWidth;
      const wide = [...document.querySelectorAll('main *')].filter(e => e.getBoundingClientRect().right > vw + 1 && !e.closest('pre, table, .table-wrapper')).map(e => e.tagName + '.' + e.className).slice(0, 5);
      const imgs = [...document.querySelectorAll('main img, main video')].map(e => Math.round(e.getBoundingClientRect().width));
      return { vw, scrollW: document.documentElement.scrollWidth, overflowingOutsideCode: wide, mediaWidths: imgs, bodyFontPx: getComputedStyle(document.querySelector('main p')).fontSize, metaViewport: document.querySelector('meta[name=viewport]')?.content };
    });
    const slug = `${dev.replace(/\s/g, '')}-${p.split('/').pop().replace('.html', '')}`;
    await shot(page, `b5-${slug}`);
    await page.locator('main img, main video').first().scrollIntoViewIfNeeded();
    await shot(page, `b5-${slug}-media`);
    out.b5.push({ dev, page: p, ...m });
  }
  await ctx.close();
}

await browser.close();
await writeFile('result.json', JSON.stringify(out, null, 2));
console.log(JSON.stringify(out, null, 2));
