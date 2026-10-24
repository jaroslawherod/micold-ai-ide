const { chromium } = require('playwright');
(async () => {
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: 1312, height: 1400 } });
  await p.goto('file:///mnt/project-files/issue-1298/design.html');
  for (const t of ['light', 'dark']) {
    await p.evaluate(t => document.documentElement.setAttribute('data-theme', t), t);
    await p.screenshot({ path: process.argv[2] + `/design-${t}.png`, fullPage: true });
    const cards = await p.$$('.card');
    for (let i = 0; i < cards.length; i++) await cards[i].screenshot({ path: process.argv[2] + `/${t}-card${i}.png` });
  }
  const info = await p.evaluate(() => [...document.querySelectorAll('.card')].map(c => { const r = c.getBoundingClientRect(); return [Math.round(r.width), Math.round(r.height)]; }));
  console.log(JSON.stringify(info));
  await b.close();
})();
