import { describe, expect, it } from "vitest";
import { gravatarUrl, md5 } from "./gravatar";

describe("md5", () => {
  it("matches known vectors", () => {
    expect(md5("")).toBe("d41d8cd98f00b204e9800998ecf8427e");
    expect(md5("abc")).toBe("900150983cd24fb0d6963f7d28e17f72");
    expect(md5("The quick brown fox jumps over the lazy dog")).toBe(
      "9e107d9d372bb6826bd81d3542a419d6",
    );
  });
});

describe("gravatarUrl", () => {
  it("lowercases + trims the email and includes the d=404 fallback", () => {
    // gravatar hashes the trimmed, lowercased email.
    const expected = md5("bruno@example.com");
    expect(gravatarUrl("  Bruno@Example.com ")).toBe(
      `https://www.gravatar.com/avatar/${expected}?s=48&d=404`,
    );
  });
});
