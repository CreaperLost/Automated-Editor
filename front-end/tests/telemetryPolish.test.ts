import test from "node:test";
import assert from "node:assert/strict";
import { cameraFromLayout, canvasFromLayout, layoutFromSettings } from "../src/lib/types";
import { cropOffFor, setCropFor } from "../src/lib/windowCrop";

test("the telemetry polish settings default on and survive a layout save", () => {
  const canvas = canvasFromLayout({ aspectRatio: "16:9", paddingPx: 0 });
  const camera = cameraFromLayout({ aspectRatio: "16:9", paddingPx: 0 });
  assert.equal(camera.autoHide, true);
  assert.equal(canvas.windowCrop, true);
  assert.equal(canvas.windowCropPaddingPct, 3);
  assert.deepEqual(canvas.windowCropOff, []);
  const off = [{ media: "rec", sourceStartUs: 1, sourceEndUs: 2 }];
  const layout = layoutFromSettings({ ...canvas, windowCrop: false, windowCropOff: off }, { ...camera, autoHide: false });
  assert.equal(layout.webcamAutoHide, false);
  assert.equal(layout.windowCrop, false);
  assert.deepEqual(layout.windowCropOff, off);
  // Saving the canvas again keeps the stretches it read.
  assert.deepEqual(canvasFromLayout(layout).windowCropOff, off);
});

test("a clip's window crop switches off and back on without touching other stretches", () => {
  const other = { media: "rec", sourceStartUs: 0, sourceEndUs: 1_000_000 };
  let ranges = setCropFor([other], "rec", 5_000_000, 8_000_000, false);
  assert.equal(cropOffFor(ranges, "rec", 5_000_000, 8_000_000), true);
  assert.equal(cropOffFor(ranges, "rec", 4_000_000, 8_000_000), false);
  ranges = setCropFor(ranges, "rec", 5_000_000, 8_000_000, true);
  assert.deepEqual(ranges, [other]);
  // A clip inside a longer stretch splits it.
  ranges = setCropFor([{ media: "rec", sourceStartUs: 0, sourceEndUs: 10 }], "rec", 4, 6, true);
  assert.deepEqual(ranges, [
    { media: "rec", sourceStartUs: 0, sourceEndUs: 4 },
    { media: "rec", sourceStartUs: 6, sourceEndUs: 10 },
  ]);
  // A stretch without media is the first recording's.
  assert.equal(cropOffFor([{ sourceStartUs: 0, sourceEndUs: 10 }], "rec", 2, 3, "rec"), true);
  assert.equal(cropOffFor([{ sourceStartUs: 0, sourceEndUs: 10 }], "other", 2, 3, "rec"), false);
});
