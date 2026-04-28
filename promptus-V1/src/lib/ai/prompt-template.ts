import type { StyleGuide } from "@/lib/engine/types";

interface BuildPromptInput {
  name: string;
  description?: string | null;
  tags: string[];
  type: string;
  styleGuide: StyleGuide;
}

/**
 * Builds an image-generation prompt by combining the campaign style guide
 * (prefix, suffix, mood, art style) with the entity's own description and tags.
 * Order matters: style prefix → entity descriptor → tags → mood → suffix.
 */
export function buildEntityImagePrompt(input: BuildPromptInput): string {
  const parts: string[] = [];

  if (input.styleGuide.promptPrefix) parts.push(input.styleGuide.promptPrefix.trim());
  if (input.styleGuide.artStyle) parts.push(input.styleGuide.artStyle);

  const subject = input.description?.trim()
    ? `${input.type}: ${input.name}, ${input.description.trim()}`
    : `${input.type}: ${input.name}`;
  parts.push(subject);

  if (input.tags.length) {
    parts.push(input.tags.slice(0, 6).join(", "));
  }

  if (input.styleGuide.mood) parts.push(input.styleGuide.mood);
  if (input.styleGuide.palette) parts.push(`palette: ${input.styleGuide.palette}`);
  if (input.styleGuide.promptSuffix) parts.push(input.styleGuide.promptSuffix.trim());

  return parts.filter(Boolean).join(", ");
}
