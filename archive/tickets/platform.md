# Archive — épic `platform`

| Ticket | Ce qui a été livré |
| --- | --- |
| `platform/scaffold-workspace` | Workspace Cargo (`back/`, `shared/`, `src-tauri/`), `front/` React + Vite + shadcn + i18n, scripts Bun, Postgres 17 de dev et base de test séparée. Le lancement sur le simulateur iOS n'a jamais été fait : abandonné |
| `platform/run-ci` | `.github/workflows/ci.yml` : fmt, clippy, tests Rust contre Postgres, lint, typecheck et tests front, gitleaks. Le premier passage sur GitHub n'a jamais été vérifié : abandonné |
| `platform/connect-claude-mcp` | Jetons personnels du MJ (hachés, révocables, limités aux routes de préparation) et serveur MCP local (`mcp/`) : lister, lire, alertes de cohérence, modifications proposées, export et import. Testé par Romain, « assez convaincu » (11 octobre 2026). Mode d'emploi : `docs/claude-mcp.md` |
| `platform/sign-in-gm` | Compte MJ, chaque route MJ réservée au MJ propriétaire de la campagne, joueurs sans compte ; vérifié dans PCT 105. La cérémonie réelle en HTTPS n'a jamais été faite : abandonné |

Clos le 11 octobre 2026 : « pas à faire, aucun intérêt » (Romain), pour
travailler d'abord sur la nouvelle création de campagne, PixelLab et le
HD-2D.
