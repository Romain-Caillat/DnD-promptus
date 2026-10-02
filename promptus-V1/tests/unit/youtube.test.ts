import { describe, it, expect } from "vitest";
import { musicPosition, parseYouTube } from "@/lib/media/youtube";

describe("parseYouTube", () => {
  it.each([
    ["https://www.youtube.com/watch?v=dQw4w9WgXcQ", { videoId: "dQw4w9WgXcQ" }],
    ["https://youtu.be/dQw4w9WgXcQ?t=42", { videoId: "dQw4w9WgXcQ" }],
    ["youtube.com/shorts/dQw4w9WgXcQ", { videoId: "dQw4w9WgXcQ" }],
    ["https://music.youtube.com/watch?v=dQw4w9WgXcQ&list=PL1234567890ab", { videoId: "dQw4w9WgXcQ", playlistId: "PL1234567890ab" }],
    ["https://www.youtube.com/playlist?list=PLx0sYbCqOb8TBPRdmBHs5Iftvv9TPboYG", { playlistId: "PLx0sYbCqOb8TBPRdmBHs5Iftvv9TPboYG" }],
    ["dQw4w9WgXcQ", { videoId: "dQw4w9WgXcQ" }],
  ])("%s", (url, expected) => expect(parseYouTube(url)).toEqual(expected));

  it("refuse les autres sites et les liens incomplets", () => {
    expect(parseYouTube("https://vimeo.com/123")).toBeNull();
    expect(parseYouTube("https://www.youtube.com/")).toBeNull();
    expect(parseYouTube("pas un lien")).toBeNull();
  });
});

describe("musicPosition", () => {
  it("avance pendant la lecture, se fige en pause", () => {
    const m = { videoId: "dQw4w9WgXcQ", startedAt: 1_000_000, offsetSec: 10, playing: true };
    expect(musicPosition(m, 1_030_000)).toBe(40);
    expect(musicPosition({ ...m, playing: false }, 1_030_000)).toBe(10);
  });
});
