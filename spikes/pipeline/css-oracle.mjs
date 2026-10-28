// Chrome's own CSS compositing as the reference: absolutely positioned divs on a white page.

/** Renders `html` at 1x and returns the RGBA of each [x, y] point. */
export async function cssPixels(browser, html, width, height, points) {
  const page = await browser.newPage({
    viewport: { width, height },
    deviceScaleFactor: 1,
  });
  try {
    await page.setContent(
      `<!doctype html><style>*{margin:0;box-sizing:border-box}div{position:absolute}</style><body style="background:#fff">${html}</body>`,
    );
    const shot = await page.screenshot({ type: "png" });
    return await page.evaluate(
      async ({ b64, points }) => {
        const img = new Image();
        img.src = "data:image/png;base64," + b64;
        await img.decode();
        const canvas = document.createElement("canvas");
        canvas.width = img.width;
        canvas.height = img.height;
        const context = canvas.getContext("2d");
        context.drawImage(img, 0, 0);
        return points.map(([x, y]) =>
          Array.from(context.getImageData(x, y, 1, 1).data),
        );
      },
      { b64: shot.toString("base64"), points },
    );
  } finally {
    await page.close();
  }
}

/** A div at x/y/width/height with extra inline style and children. */
export const div = (x, y, w, h, style = "", inner = "") =>
  `<div style="left:${x}px;top:${y}px;width:${w}px;height:${h}px;${style}">${inner}</div>`;
