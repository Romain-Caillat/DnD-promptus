// Checks that every rule, platform fiche and ticket page follows its
// template (docs/README.md): fields in order, a valid state, short lists
// and schemas, one screen per page, and links that point both ways.
// Usage: bun scripts/check-docs.ts

import { Glob } from "bun";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join, normalize, relative } from "node:path";

const ROOT = join(import.meta.dir, "..");
const MAX_PAGE_LINES = 45;
const MAX_SCHEMA_LINES = 15;
const MAX_BULLETS = 5;

type Kind = "rule" | "fiche" | "ticket";

// Each template is a list of steps; a step is a field, an optional
// field, or a choice between field sequences.
type Step = string | { optional: string } | { oneOf: string[][] };

const TEMPLATES: Record<Kind, Step[]> = {
  rule: [
    "État",
    { oneOf: [["Règle"], ["Question", "Options", "Recommandation"]] },
    "Schéma",
    { optional: "Détails" },
    "Pourquoi",
    "Impacts",
    "Résolue par",
  ],
  fiche: ["État", "Résout", "Solution", "Schéma", "Comment", "Limites", "Tickets"],
  ticket: [
    "État",
    "Réalise",
    "Objectif",
    { optional: "Schéma" },
    "Périmètre",
    "Fini quand",
    "Départ",
  ],
};

const STATES: Record<Kind, RegExp> = {
  rule: /^(à définir|provisoire · .+|tranchée · .+)$/,
  fiche: /^(à définir|provisoire · .+|tranchée · .+)$/,
  ticket: /^(à spécifier|à faire|en cours|bloqué · .+)( · .+)?$/,
};

function kindOf(path: string): Kind | null {
  if (path.endsWith("README.md")) return null;
  if (path.startsWith("docs/regles/")) return "rule";
  if (path.startsWith("docs/plateforme/")) return "fiche";
  if (path.startsWith("docs/tickets/")) return "ticket";
  return null;
}

interface Page {
  path: string;
  kind: Kind;
  fields: Map<string, string>;
  order: string[];
}

const errors: string[] = [];
const fail = (path: string, message: string) => errors.push(`${path}: ${message}`);

function parse(path: string, kind: Kind): Page {
  const lines = readFileSync(join(ROOT, path), "utf8").split("\n");
  const fields = new Map<string, string>();
  const order: string[] = [];

  if (!lines[0]?.startsWith("# ")) fail(path, "the first line must be the title (# Nom)");
  if (lines.length > MAX_PAGE_LINES + 1) fail(path, `${lines.length} lines, at most ${MAX_PAGE_LINES}`);

  let current = "";
  let fence: number | null = null;
  let bullets = 0;
  lines.forEach((line, i) => {
    if (line.includes("§")) fail(path, `line ${i + 1}: no « § », name the section`);
    if (line.startsWith("```")) {
      if (fence === null) fence = i;
      else {
        if (i - fence - 1 > MAX_SCHEMA_LINES)
          fail(path, `schema at line ${fence + 1}: ${i - fence - 1} lines, at most ${MAX_SCHEMA_LINES}`);
        fence = null;
      }
      return;
    }
    if (fence !== null) return;
    bullets = line.startsWith("- ") ? bullets + 1 : line.trim() === "" ? bullets : 0;
    if (bullets > MAX_BULLETS) {
      fail(path, `line ${i + 1}: more than ${MAX_BULLETS} bullets`);
      bullets = 0;
    }
    const field = /^\*\*([^*]+)\*\* :\s?(.*)$/.exec(line);
    if (field) {
      current = field[1];
      order.push(current);
      fields.set(current, field[2]);
      if (i > 0 && lines[i - 1].trim() !== "") fail(path, `line ${i + 1}: a blank line must precede « ${current} »`);
    } else if (current && line.trim() !== "") {
      fields.set(current, `${fields.get(current)}\n${line}`);
    }
  });
  return { path, kind, fields, order };
}

function checkOrder(page: Page) {
  const steps = TEMPLATES[page.kind];
  let at = 0;
  const take = (name: string) => {
    if (page.order[at] === name) {
      at++;
      return true;
    }
    return false;
  };
  for (const step of steps) {
    if (typeof step === "string") {
      if (!take(step)) return fail(page.path, `expected « ${step} », found « ${page.order[at] ?? "end of page"} »`);
    } else if ("optional" in step) {
      take(step.optional);
    } else {
      const branch = step.oneOf.find((seq) => seq[0] === page.order[at]);
      if (!branch) return fail(page.path, `expected one of ${step.oneOf.map((s) => `« ${s.join(" · ")} »`).join(" or ")}`);
      for (const name of branch) if (!take(name)) return fail(page.path, `expected « ${name} »`);
    }
  }
  if (at < page.order.length) fail(page.path, `unexpected field « ${page.order[at]} »`);
}

// Relative markdown links of a field, resolved to repository paths.
function links(page: Page, field: string): string[] {
  const text = page.fields.get(field) ?? "";
  return [...text.matchAll(/\]\(([^)#]+\.md)\)/g)].map((m) => normalize(join(dirname(page.path), m[1])));
}

const pages = new Map<string, Page>();
for (const path of new Glob("docs/{regles,plateforme,tickets}/**/*.md").scanSync(ROOT)) {
  const kind = kindOf(path);
  if (!kind) continue;
  const page = parse(path, kind);
  checkOrder(page);
  const state = page.fields.get("État")?.trim() ?? "";
  if (!STATES[kind].test(state)) fail(path, `state « ${state} » is not one of the template's`);
  pages.set(path, page);
}

// Every link of every page must exist; the chains must point both ways.
const BACK_LINKS: [Kind, string, Kind, string][] = [
  ["rule", "Résolue par", "fiche", "Résout"],
  ["fiche", "Tickets", "ticket", "Réalise"],
];
for (const page of pages.values()) {
  for (const field of page.fields.keys()) {
    for (const target of links(page, field)) {
      if (!existsSync(join(ROOT, target))) fail(page.path, `« ${field} » links to a missing ${relative("docs", target)}`);
    }
  }
  for (const [kind, field, otherKind, otherField] of BACK_LINKS) {
    if (page.kind !== kind) continue;
    for (const target of links(page, field)) {
      const other = pages.get(target);
      if (other?.kind === otherKind && !links(other, otherField).includes(page.path))
        fail(page.path, `« ${field} » → ${target}, whose « ${otherField} » does not link back`);
    }
  }
}

if (errors.length) {
  console.error(errors.join("\n"));
  console.error(`\n${errors.length} problem(s) in ${pages.size} pages`);
  process.exit(1);
}
console.log(`${pages.size} pages follow their template`);
