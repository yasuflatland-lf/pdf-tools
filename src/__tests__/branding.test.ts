import { describe, expect, it } from "vitest";
import indexHtml from "../../index.html?raw";
import logoSvg from "../../public/logo.svg?raw";

/**
 * The logo lives in `public/` as a single file that both the favicon and the
 * toolbar reach by URL, so nothing type-checks either reference. These tests
 * stand in for that, and they guard the framing the bundle icons are generated
 * from: a shield that leaves the viewBox is a shield cropped in every icon.
 */
describe("branding assets", () => {
  it("serves the logo as the document favicon", () => {
    expect(indexHtml).toContain('href="/logo.svg"');
  });

  it("keeps every path inside the logo's viewBox", () => {
    const viewBox = /viewBox="(-?[\d.]+) (-?[\d.]+) ([\d.]+) ([\d.]+)"/.exec(logoSvg);
    if (!viewBox) {
      throw new Error("The logo declares no viewBox");
    }
    const [minX, minY, width, height] = viewBox.slice(1).map(Number);

    const paths = [...logoSvg.matchAll(/ d="([^"]+)"/g)].map((match) => match[1]);
    expect(paths.length).toBeGreaterThan(0);

    for (const path of paths) {
      // Reading the numbers as x/y pairs only holds while the path is written
      // in absolute commands, so refuse anything else rather than mis-measure.
      const commands = [...path.matchAll(/[A-Za-z]/g)].map((match) => match[0]);
      expect(commands.every((command) => "MLCZ".includes(command))).toBe(true);

      const numbers = [...path.matchAll(/-?[\d.]+/g)].map((match) => Number(match[0]));
      const xs = numbers.filter((_, index) => index % 2 === 0);
      const ys = numbers.filter((_, index) => index % 2 === 1);

      expect(Math.min(...xs)).toBeGreaterThanOrEqual(minX);
      expect(Math.max(...xs)).toBeLessThanOrEqual(minX + width);
      expect(Math.min(...ys)).toBeGreaterThanOrEqual(minY);
      expect(Math.max(...ys)).toBeLessThanOrEqual(minY + height);
    }
  });
});
