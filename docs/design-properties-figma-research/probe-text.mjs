// Research-only probe; no backend, user project or generated product bundle.
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import path from 'node:path';
import { build } from 'esbuild';

process.env.PLAYWRIGHT_BROWSERS_PATH ??= path.resolve('.runtime/browsers');
const { chromium } = await import('playwright');
const fontBytes = readFileSync('renderer/fonts/NotoSans.ttf');
const source = await build({
  entryPoints: ['renderer/measurement.ts'], bundle: true,
  format: 'iife', globalName: 'probeMeasurement', write: false,
});
const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage();
  await page.setContent('<style>body {margin:0}</style>');
  await page.addScriptTag({ content: source.outputFiles[0].text });
  const results = await page.evaluate(async (base64) => {
    const bytes = Uint8Array.from(atob(base64), c => c.charCodeAt(0));
    const face = new FontFace('Research Noto', bytes, { weight: '400', style: 'normal' });
    document.fonts.add(await face.load());
    const font = {id:'probe', family:'Research Noto', hash:'probe', weight:400, style:'normal'};
    const resources = {fonts:new Map([['probe',font]])};
    const measure = (text, width = null, size = 14, lineHeight = 20) =>
      probeMeasurement.measureBatch([{...font, key:'r', fontId:'probe', fontHash:'probe',
        text, width, fontSize:size, lineHeight}], resources).r;
    const context = document.createElement('canvas').getContext('2d');
    context.font = '400 14px "Research Noto"';
    context.fontKerning = 'normal';
    const widths = ['Prefer', 'customers', 'Sao chép', 'exploring', 'AVATAR office'].map(text => {
      const raw = context.measureText(text).width;
      const measured = measure(text);
      const div = document.createElement('div');
      div.style.cssText = 'font:400 14px/20px "Research Noto";width:max-content;white-space:pre';
      div.textContent = text;
      document.body.append(div);
      const dom = div.getBoundingClientRect().width;
      div.remove();
      return {text, raw, dom, measured:measured.width,
        remeasureLines:measure(text, measured.width).lines.map(l => l.text)};
    });
    const baselines = [[14,20],[32,40],[15,15]].map(([size,lineHeight]) => {
      const div = document.createElement('div');
      div.style.cssText = `font:400 ${size}px/${lineHeight}px "Research Noto";width:max-content`;
      div.append(document.createTextNode('Tiếng Việt'));
      const marker = document.createElement('span');
      marker.style.cssText = 'display:inline-block;width:0;height:0;vertical-align:baseline';
      div.append(marker);
      document.body.append(div);
      const domBaseline = marker.getBoundingClientRect().top - div.getBoundingClientRect().top;
      div.remove();
      return {size,lineHeight, domBaseline, sceneBaseline:measure('Tiếng Việt',null,size,lineHeight).lines[0].y};
    });
    const whitespace = ['A  B',' A\tB ','A\u00a0B','A\nB','A\u202fB'].map(text => {
      const div = document.createElement('div');
      div.style.cssText = 'font:400 14px/20px "Research Noto";width:20px;white-space:pre-line';
      div.textContent = text;
      document.body.append(div);
      const domHeight = div.getBoundingClientRect().height;
      div.remove();
      return {input:text, lines:measure(text,20).lines.map(l => l.text), domHeight};
    });
    const tracking = [0, 0.17*14, -0.055*14].map(spacing => {
      const text = 'Your data, without the noise.';
      context.letterSpacing = `${spacing}px`;
      const width = context.measureText(text).width;
      const sumCharacters = [...text].reduce((sum,c) => sum + context.measureText(c).width,0);
      return {spacing, wholeRunWidth:width, sumCharacterWidths:sumCharacters};
    });
    return {widths,baselines,whitespace,tracking,
      letterSpacingAvailable:'letterSpacing' in context};
  }, fontBytes.toString('base64'));
  console.log(JSON.stringify({
    revision:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim(),
    platform:process.platform, node:process.version, chromium:browser.version(),
    fontSha256:createHash('sha256').update(fontBytes).digest('hex'),
    measurementSha256:createHash('sha256').update(readFileSync('renderer/measurement.ts')).digest('hex'),
    ...results,
  },null,2));
} finally {
  await browser.close();
}
