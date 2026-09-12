#!/usr/bin/env node
// The rendered-page checks (feature 028, T035).
//
//   site/checks/page-checks.mjs DIR [--home index.html]
//
// Everything else in the publication reads text. This is the only check that renders the page in a
// browser, which is the only place three of the site's promises can be observed at all:
//
//   * WCAG 2.2 AA (FR-027a, SC-013). Contrast, reachable names and focus visibility are properties
//     of a *rendered* page -- of what the cascade and the JavaScript finally produced -- not of the
//     markup that led there. axe-core is the reference implementation of the rule set.
//   * The first screen (FR-023a, SC-001). "Above the fold" is a geometric fact about a viewport,
//     and there is no way to hold a page to it without laying it out.
//   * Nothing off-origin (FR-027a, SC-015). A page that fetches a font, a stylesheet or a script
//     from another host sends every reader's address to that host and reads differently when the
//     host is unreachable. Both the markup and the requests the browser actually makes are checked,
//     because an `@import` inside a stylesheet is invisible to the first and obvious to the second.
//   * The distance between pages (FR-023a, SC-006). The links a reader can follow are the ones the
//     rendered page offers -- mdBook writes the sidebar from a script, so the markup on disk holds
//     almost none of them.
//   * Nothing moving (FR-015a, FR-028, SC-011). A clip has to hold still behind its poster until
//     the reader presses play, and fetch no video bytes before that. The attributes say what the
//     page intends; the requests the browser made say what it did.
//   * Search (FR-026, SC-006). The answer is produced by the site's own index and its own ranking,
//     in the box the reader types into, so the only way to ask what a reader would be told is to
//     type the question and read what comes back.
//
// Pages are served over loopback rather than opened as files: a `file://` page has an opaque origin,
// so "another origin" would mean nothing and the off-origin check would pass everything.

import { createServer } from 'node:http';
import { readFile, readdir, stat } from 'node:fs/promises';
import { extname, join, relative, resolve, sep } from 'node:path';
import { chromium } from 'playwright';
import AxeBuilder from '@axe-core/playwright';

const TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.json': 'application/json',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.svg': 'image/svg+xml',
  '.webm': 'video/webm',
  '.mp4': 'video/mp4',
  '.ttf': 'font/ttf',
  '.woff2': 'font/woff2',
  '.txt': 'text/plain; charset=utf-8',
};

// The viewports the first-screen facts are asserted at: a laptop and the narrowest phone the site
// claims to work on (FR-025). A fact that is on the first screen of one and not the other has not
// been kept.
const VIEWPORTS = [
  { name: 'laptop', width: 1280, height: 800 },
  { name: 'phone', width: 360, height: 640 },
];

// mdBook's own theme classes, which the site mirrors onto `data-scheme`. Setting the stored theme
// before the page loads is how the check reaches the dark scheme: it is the same switch the reader
// uses, so what is measured is the page as they would see it.
const SCHEMES = [
  { name: 'light', theme: 'light', colorScheme: 'light' },
  { name: 'dark', theme: 'navy', colorScheme: 'dark' },
];

// How far apart two pages of a documentation site may be (FR-023a, SC-006). One step is a link on
// the page in front of the reader; two is that link's page offering the next. At three the reader is
// following a corridor and has stopped believing the page they want is at the end of it.
const NAV_MAX_STEPS = 2;

// Files that are not pages of the book, and so are not part of the question "how far is this page
// from that one": the host serves `404.html` when a URL resolves to nothing, and `toc.html` is the
// navigation itself -- a fragment fetched by script, never opened, and linked from nowhere.
const NOT_NAVIGABLE = new Set(['404.html', 'toc.html', 'print.html']);

// Where the guide lives. Every page under it is a topic a reader arrives already knowing the name
// of, which is exactly the query the search box has to answer with that page and not another.
const GUIDE_PREFIX = 'user-guide/';

function parseArgs(argv) {
  const args = { site: null, home: 'index.html' };
  for (let i = 0; i < argv.length; i += 1) {
    switch (argv[i]) {
      // `--site` and the bare directory are the same argument. `site/build.sh` passes the built
      // directory positionally, as contracts/site-checks.md spells the check; the named form reads
      // better in a test that is spelling out what it is pointing at.
      case '--site': args.site = argv[++i]; break;
      case '--home': args.home = argv[++i]; break;
      case '-h':
      case '--help':
        console.log('usage: page-checks.mjs DIR [--home index.html]');
        process.exit(0);
        break;
      default:
        if (argv[i].startsWith('-') || args.site) throw new Error(`unknown argument: ${argv[i]}`);
        args.site = argv[i];
    }
  }
  if (!args.site) throw new Error('the directory of the built site is required');
  return args;
}

async function serve(root) {
  const server = createServer(async (req, res) => {
    const path = decodeURIComponent(new URL(req.url, 'http://127.0.0.1').pathname);
    let file = resolve(root, `.${path}`);
    // Never serve outside the site: a check that can read the whole disk is a check that can be
    // pointed at one by a crafted link in a fixture.
    if (!file.startsWith(resolve(root))) {
      res.writeHead(403).end();
      return;
    }
    try {
      if ((await stat(file)).isDirectory()) file = join(file, 'index.html');
      const body = await readFile(file);
      res.writeHead(200, { 'content-type': TYPES[extname(file)] ?? 'application/octet-stream' });
      res.end(body);
    } catch {
      res.writeHead(404, { 'content-type': 'text/plain' }).end('not found');
    }
  });
  await new Promise((ok) => server.listen(0, '127.0.0.1', ok));
  return { server, origin: `http://127.0.0.1:${server.address().port}` };
}

async function pages(root) {
  const found = [];
  async function walk(dir) {
    for (const entry of await readdir(dir, { withFileTypes: true })) {
      const full = join(dir, entry.name);
      if (entry.isDirectory()) {
        if (entry.name === 'node_modules') continue;
        await walk(full);
      } else if (entry.name.endsWith('.html')) {
        found.push(relative(root, full).split(sep).join('/'));
      }
    }
  }
  await walk(root);
  return found.sort();
}

const failures = [];
const fail = (where, message) => failures.push(`${where}: ${message}`);

// --- the off-origin scan -------------------------------------------------------------------------

// Read out of the rendered page: every URL the markup points at, plus every `url()` in every
// stylesheet the page managed to load. Same-document schemes (`data:`, `blob:`, `about:`) are not
// another host and are left alone.
const collectReferences = () => {
  const refs = [];
  const add = (value, from) => {
    if (!value) return;
    for (const part of String(value).split(',')) {
      const url = part.trim().split(/\s+/)[0];
      if (!url || url.startsWith('data:') || url.startsWith('blob:') || url.startsWith('about:')) continue;
      try {
        refs.push({ url: new URL(url, document.baseURI).href, from });
      } catch { /* a URL the browser itself cannot parse is not a request to another host */ }
    }
  };
  for (const el of document.querySelectorAll('img[src], script[src], source[src], video[poster], audio[src], iframe[src]')) {
    add(el.getAttribute('src') || el.getAttribute('poster'), el.tagName.toLowerCase());
  }
  for (const el of document.querySelectorAll('img[srcset], source[srcset]')) {
    add(el.getAttribute('srcset'), `${el.tagName.toLowerCase()}[srcset]`);
  }
  for (const el of document.querySelectorAll('link[href]')) {
    const rel = (el.getAttribute('rel') || '').toLowerCase();
    // A `<link rel="stylesheet">` or a preloaded font is fetched; `rel="canonical"` and friends are
    // addresses, not requests, and a documentation site is allowed to name other pages.
    if (/stylesheet|preload|preconnect|dns-prefetch|prefetch|icon|manifest/.test(rel)) {
      add(el.getAttribute('href'), `link[rel=${rel}]`);
    }
  }
  for (const sheet of document.styleSheets) {
    let rules;
    try { rules = sheet.cssRules; } catch { continue; } // a sheet from another origin cannot be read
    if (sheet.href) add(sheet.href, 'stylesheet');
    const walk = (list) => {
      for (const rule of list) {
        if (rule.cssRules) walk(rule.cssRules);
        if (rule.href) add(rule.href, '@import');
        const text = rule.style ? rule.cssText : '';
        for (const match of text.matchAll(/url\(\s*['"]?([^'")]+)['"]?\s*\)/g)) add(match[1], 'css url()');
      }
    };
    walk(rules);
  }
  return refs;
};

// --- nothing moves until the reader asks ----------------------------------------------------------

// A clip on this site is a poster with a play control (FR-015a) that fetches nothing until it is
// pressed (FR-028). Three attributes carry that, and any one of them can be dropped in an edit
// without breaking anything a link check, a budget check or a contrast check would notice: the page
// stays valid and starts moving at a reader who did not ask.
//
// Animated GIFs are refused for the same reason and one more: a GIF has no play control at all, so
// a reader who wants it to stop has no way to say so. Clips are video here precisely because of it
// (research §7).
const collectMotion = () => {
  const problems = [];
  const name = (el) => el.getAttribute('aria-label') || el.getAttribute('poster') || el.id || '<video>';

  for (const video of document.querySelectorAll('video')) {
    const who = name(video);
    if (video.hasAttribute('autoplay')) problems.push(`the clip "${who}" carries autoplay, so it starts without the reader`);
    if (!video.hasAttribute('controls')) problems.push(`the clip "${who}" has no controls, so the reader cannot start or stop it`);
    if (!video.hasAttribute('muted') && !video.muted) problems.push(`the clip "${who}" is not muted`);
    const preload = (video.getAttribute('preload') || '').toLowerCase();
    if (preload !== 'none') {
      problems.push(`the clip "${who}" has preload="${preload || '(unset)'}", so its video is fetched before the reader asks -- it must be preload="none"`);
    }
    if (!video.hasAttribute('poster')) problems.push(`the clip "${who}" has no poster, so there is no still first frame to show`);
  }

  if (document.querySelectorAll('audio').length > 0) {
    problems.push('the page carries an <audio> element, and the site has no sound');
  }

  for (const img of document.querySelectorAll('img[src]')) {
    if (/\.gif(\?|$)/i.test(img.getAttribute('src') || '')) {
      problems.push(`${img.getAttribute('src')} is an animated image with no way to pause it -- clips are video here`);
    }
  }

  // What is actually running, rather than what is declared: a stylesheet with an endless animation
  // moves the page whatever the markup says. Finite animations are the site's own entrance
  // transitions and stop on their own.
  for (const animation of document.getAnimations()) {
    const timing = animation.effect?.getTiming?.() ?? {};
    if (animation.playState === 'running' && timing.iterations === Infinity) {
      const target = animation.effect?.target;
      problems.push(`something on the page animates forever: ${target?.tagName?.toLowerCase() ?? '?'}${target?.id ? `#${target.id}` : ''}`);
    }
  }

  return problems;
};

// --- the first screen ----------------------------------------------------------------------------

// What a visitor has to be able to do without scrolling (FR-023a): name the thing, see it, and reach
// both the installation instructions and the guide. Each fact is an element that has to be *in* the
// first viewport -- not merely present on the page.
//
// Each `find` returns candidates in preference order rather than one element, because the same words
// appear twice on the page: the sidebar lists "Installing Micold AI IDE" and the prose links to it,
// and on a phone the sidebar is the hidden one. Measuring whichever came first in document order
// would have the check report a link as missing from the first screen while the reader is looking
// straight at it.
const FACTS = [
  {
    what: 'the product name',
    find: () => [...document.querySelectorAll('main h1, .content h1, h1, .menu-title')],
  },
  {
    // The site's whole subject is an application with a window; a home page that describes it
    // without showing it is the failure this one exists to catch.
    what: 'an image of the application',
    find: () => [...document.querySelectorAll('main img, .content img, img')],
  },
  {
    what: 'a link to the install page',
    find: () => [...document.querySelectorAll('main a[href], .content a[href], a[href]')]
      .filter((a) => /install/i.test(a.getAttribute('href')) || /install/i.test(a.textContent)),
  },
  {
    what: 'a link to the user guide',
    find: () => [...document.querySelectorAll('main a[href], .content a[href], a[href]')]
      .filter((a) => /user-guide|user_guide/i.test(a.getAttribute('href')) || /user guide/i.test(a.textContent)),
  },
];

const measureFacts = (facts) => facts.map((fact) => {
  // eslint-disable-next-line no-new-func
  const candidates = new Function(`return (${fact.find})()`)();
  if (candidates.length === 0) return { what: fact.what, present: false };
  const shown = candidates
    .map((el) => ({ el, box: el.getBoundingClientRect() }))
    .filter(({ box }) => box.width > 0 && box.height > 0);
  if (shown.length === 0) return { what: fact.what, present: true, displayed: false };
  // Measured against the viewport the page was laid out in, from the top of the document: the
  // element has to start within the first screen, and something has to be drawn there.
  const top = Math.min(...shown.map(({ box }) => box.top));
  return { what: fact.what, present: true, displayed: true, onFirstScreen: top >= 0 && top < window.innerHeight, top: Math.round(top) };
});

// --- the navigation ------------------------------------------------------------------------------

// Every link the rendered page offers, resolved. Hidden ones count: a sidebar the reader has not
// opened yet is still a sidebar, and the toggle that opens it is on the page.
const collectLinks = () => [...document.querySelectorAll('a[href]')].map((a) => {
  try { return new URL(a.getAttribute('href'), document.baseURI).href; } catch { return null; }
}).filter(Boolean);

// A link, as a page of this site -- or null if it leads somewhere that is not one. Fragments and
// query strings are dropped: `settings.html#terminal` and `settings.html?highlight=x` are the same
// page arrived at differently, and mdBook's own search writes both.
function asPage(href, origin, known) {
  let url;
  try { url = new URL(href); } catch { return null; }
  if (url.origin !== origin) return null;
  let path = decodeURIComponent(url.pathname).replace(/^\//, '');
  if (path === '' || path.endsWith('/')) path += 'index.html';
  return known.has(path) ? path : null;
}

// How many links a reader has to follow to get from `from` to each other page. Plain breadth-first
// search: the answer is the number of steps, and a page the search never reaches has no answer.
function distances(from, graph) {
  const seen = new Map([[from, 0]]);
  const queue = [from];
  while (queue.length > 0) {
    const here = queue.shift();
    for (const next of graph.get(here) ?? []) {
      if (seen.has(next)) continue;
      seen.set(next, seen.get(here) + 1);
      queue.push(next);
    }
  }
  return seen;
}

// --- search --------------------------------------------------------------------------------------

// Ask the site's own search box a question whose right answer is known, the way a reader asks it:
// click into the box, type, and read the first thing offered back. Nothing here reaches into the
// index or calls the searcher -- both would test a different program than the one on the page.
async function firstSearchResult(page, origin, known, query) {
  const bar = page.locator('#mdbook-searchbar');
  if ((await bar.count()) === 0) return { missing: true };
  // The box starts folded away behind the magnifier in the app bar, so opening it is the reader's
  // first move and has to be the check's. A site that keeps its box open has no toggle to click.
  if (!(await bar.isVisible())) {
    const toggle = page.locator('#mdbook-search-toggle');
    if ((await toggle.count()) > 0) await toggle.click();
    if (!(await bar.isVisible())) return { hidden: true };
  }
  await bar.click();
  await bar.fill('');
  // Typed key by key, because the search runs on the keystroke: a value set in one go changes the
  // box without ever telling the page that the reader asked for anything.
  await bar.pressSequentially(query, { delay: 20 });
  try {
    await page.waitForFunction(
      () => document.querySelectorAll('#mdbook-searchresults li a[href]').length > 0,
      undefined,
      { timeout: 5000 },
    );
  } catch { return { empty: true }; }
  const href = await page.evaluate(
    () => document.querySelector('#mdbook-searchresults li a[href]').href,
  );
  return { href, page: asPage(href, origin, known) };
}

// --- the run -------------------------------------------------------------------------------------

const args = parseArgs(process.argv.slice(2));
const root = resolve(args.site);
const { server, origin } = await serve(root);
const all = await pages(root);
if (all.length === 0) {
  console.error(`page-checks: no pages under ${root}`);
  server.close();
  process.exit(1);
}

const known = new Set(all);
const links = new Map();
const titles = new Map();

const browser = await chromium.launch();
try {
  for (const scheme of SCHEMES) {
    const context = await browser.newContext({
      colorScheme: scheme.colorScheme,
      viewport: { width: VIEWPORTS[0].width, height: VIEWPORTS[0].height },
    });
    await context.addInitScript((theme) => {
      try { localStorage.setItem('mdbook-theme', theme); } catch { /* private mode */ }
    }, scheme.theme);

    for (const path of all) {
      const page = await context.newPage();
      const offOrigin = new Set();
      const fetchedMedia = new Set();
      page.on('request', (request) => {
        const url = new URL(request.url());
        if ((url.protocol === 'http:' || url.protocol === 'https:') && url.origin !== origin) {
          offOrigin.add(url.href);
        }
        // Nobody has pressed anything on this page: every request here was made by loading it. A
        // video among them is FR-028 broken in the only way that matters -- bytes on the wire.
        if (request.resourceType() === 'media' || /\.(webm|mp4|mov)(\?|$)/i.test(url.pathname)) {
          fetchedMedia.add(url.pathname);
        }
      });

      const where = `${path} (${scheme.name})`;
      await page.goto(`${origin}/${path}`, { waitUntil: 'load' });

      const results = await new AxeBuilder({ page })
        .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'])
        .analyze();
      for (const violation of results.violations) {
        const target = violation.nodes[0]?.target?.join(' ') ?? '?';
        fail(where, `${violation.id} (${violation.impact}) on ${target} -- ${violation.help}`);
      }

      for (const ref of await page.evaluate(collectReferences)) {
        const url = new URL(ref.url);
        if ((url.protocol === 'http:' || url.protocol === 'https:') && url.origin !== origin) {
          fail(where, `${ref.from} points at another host: ${ref.href ?? ref.url}`);
        }
      }
      for (const url of offOrigin) fail(where, `the page requested another host: ${url}`);

      for (const problem of await page.evaluate(collectMotion)) fail(where, problem);
      for (const path of fetchedMedia) fail(where, `the page fetched ${path} before the reader pressed anything (FR-028)`);

      // The link graph and the page names are the same whichever scheme the reader is in, so they
      // are read once, on the pass that is already open on every page.
      if (scheme === SCHEMES[0]) {
        const targets = new Set();
        for (const href of await page.evaluate(collectLinks)) {
          const target = asPage(href, origin, known);
          if (target && target !== path) targets.add(target);
        }
        links.set(path, [...targets]);
        // The heading of the page itself, not the site's name in the app bar -- which is an `<h1>`
        // too, and an earlier one, so a single selector list would hand back the same answer for
        // every page in the book.
        titles.set(path, (await page.evaluate(
          () => (document.querySelector('#mdbook-content h1, main h1, .content h1')
            ?? document.querySelector('h1'))?.textContent ?? '',
        )).trim());
      }

      await page.close();
    }
    await context.close();

    // The first screen is measured in a browser that was *opened* at each size, never in one that
    // was resized: the page decides at load whether the sidebar starts open, from the width it finds
    // then. Resizing a laptop-sized page down to a phone leaves that decision behind and measures a
    // layout no reader ever sees -- a sidebar-wide column with the content squeezed beside it.
    for (const viewport of VIEWPORTS) {
      const sized = await browser.newContext({
        colorScheme: scheme.colorScheme,
        viewport: { width: viewport.width, height: viewport.height },
      });
      await sized.addInitScript((theme) => {
        try { localStorage.setItem('mdbook-theme', theme); } catch { /* private mode */ }
      }, scheme.theme);
      const page = await sized.newPage();
      // `networkidle` rather than `load`: the home page's screenshot is lazily loaded, and an image
      // still in flight has no size yet -- which would make "is it on the first screen" a race.
      await page.goto(`${origin}/${args.home}`, { waitUntil: 'networkidle' });
      const facts = await page.evaluate(
        measureFacts,
        FACTS.map((f) => ({ what: f.what, find: f.find.toString() })),
      );
      for (const fact of facts) {
        const where = `${args.home} (${scheme.name}, ${viewport.name})`;
        if (!fact.present) {
          fail(where, `${fact.what} is not on the home page at all`);
        } else if (!fact.displayed) {
          fail(where, `${fact.what} is on the home page but nothing of it is drawn at ${viewport.width}x${viewport.height}`);
        } else if (!fact.onFirstScreen) {
          fail(where, `${fact.what} is not on the first screen at ${viewport.width}x${viewport.height}; it starts ${fact.top}px down`);
        }
      }
      await sized.close();
    }
  }

  // --- how far apart the pages are (FR-023a, SC-006) ---------------------------------------------
  //
  // Not "is every page reachable" -- a chain of pages is reachable and unusable. The claim is that
  // from wherever the reader is, any other page of the guide is one link away, or one link and one
  // more. Both directions matter, so every ordered pair is asked about: a page every other links to
  // and that links back to nothing is a dead end even though it is easy to find.
  const navigable = all.filter((path) => !NOT_NAVIGABLE.has(path));
  const graph = new Map(navigable.map(
    (path) => [path, (links.get(path) ?? []).filter((target) => !NOT_NAVIGABLE.has(target))],
  ));
  const far = [];
  for (const from of navigable) {
    const reach = distances(from, graph);
    for (const to of navigable) {
      if (to === from) continue;
      const steps = reach.get(to);
      if (steps === undefined) far.push(`${to} cannot be reached from ${from} by following links at all`);
      else if (steps > NAV_MAX_STEPS) far.push(`${to} is ${steps} steps from ${from}, and the site promises at most ${NAV_MAX_STEPS}`);
    }
  }
  // One line per pair would bury the shape of the problem: a single page buried three deep is
  // reported once per page it is buried from. The first few name it; the count says how wide it is.
  for (const line of far.slice(0, 6)) fail('the navigation', line);
  if (far.length > 6) fail('the navigation', `and ${far.length - 6} more pair(s) of pages that far apart`);

  // --- what the search box answers (FR-026, SC-006) ----------------------------------------------
  //
  // A reader who searches for a topic of the guide has named the page they want. Anything but that
  // page at the top of the results is the search quietly sending them somewhere else -- which is
  // how a documentation search fails in practice: not with an error, with a plausible wrong answer.
  const topics = navigable.filter((path) => path.startsWith(GUIDE_PREFIX));
  if (topics.length > 0) {
    const context = await browser.newContext({ viewport: { width: VIEWPORTS[0].width, height: VIEWPORTS[0].height } });
    const page = await context.newPage();
    for (const topic of topics) {
      // The page's own heading is the query: it is what the topic is called on the site, and so
      // what a reader who has heard of it types.
      const query = (titles.get(topic) ?? '').replace(/[^\p{L}\p{N}]+/gu, ' ').trim();
      if (query === '') {
        fail('search', `${topic} has no heading to search for`);
        continue;
      }
      await page.goto(`${origin}/${args.home}`, { waitUntil: 'load' });
      const answer = await firstSearchResult(page, origin, known, query);
      if (answer.missing) {
        fail('search', `there is no search box on ${args.home} to ask "${query}"`);
        break;
      }
      if (answer.hidden) {
        fail('search', `the search box on ${args.home} never became usable, so "${query}" could not be asked`);
        break;
      }
      if (answer.empty) fail('search', `a search for "${query}" answered with nothing, and ${topic} is the page it names`);
      else if (answer.page !== topic) fail('search', `a search for "${query}" answered with ${answer.page ?? answer.href} first, not ${topic}`);
    }
    await context.close();
  }
} finally {
  await browser.close();
  server.close();
}

if (failures.length > 0) {
  console.error(`page-checks: ${failures.length} problem(s) over ${all.length} page(s):`);
  for (const failure of failures) console.error(`  ${failure}`);
  process.exit(1);
}
console.log(`page-checks: ${all.length} page(s), both schemes -- WCAG 2.2 AA, the first screen, nothing off-origin, no page more than ${NAV_MAX_STEPS} links away, search that answers with the page it was asked about, and nothing that moves until the reader asks`);
