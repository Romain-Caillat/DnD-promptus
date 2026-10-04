"""Generate the GM prep storyboard (Preparation-MJ.dc.html): the ten moments of preparing a campaign.

Each frame is the Preparer-MJ board with seul=true, frozen on a moment once the players have acted.
Run from the repository root.
"""
import json, sys

FRAMES = [
    ('Mes campagnes', 'Retrouve ses campagnes et en commence une nouvelle.', 'Rien encore.', 'Une seule action sur l’écran d’accueil : commencer.'),
    ('L’univers', 'Choisit la fantasy : décors, personnages, objets, noms des stats.', 'Rien : un univers est un pack, pas une génération.', 'Les tuiles et les personnages d’une campagne viennent du même pack.'),
    ('Les règles', 'Garde D&D 5e (SRD) et voit les cartes que ses joueurs auront.', 'Rien encore.', 'Les règles sont des données : l’interface des joueurs en découle.'),
    ('Le pitch', 'Écrit son idée en trois phrases, choisit le ton et la durée.', 'Rien : il voit le coût estimé avant de lancer.', 'Aucun appel payant ne part sans prix affiché.'),
    ('La génération', 'Suit les étapes et lit la bible pendant que la suite s’écrit.', 'Écrit la bible, les fronts, les scènes, les fiches, les indices.', 'Attendre en lisant plutôt que devant une barre de chargement.'),
    ('L’atelier', 'Demande un maire plus ambigu et tranche sur le diff proposé.', 'Propose 2 changements, chiffrés, que le MJ accepte ou rejette.', 'Le graphe au centre, la conversation à gauche, la scène à droite.'),
    ('La cohérence', 'Voit qu’une révélation n’a que 2 indices et ajoute le troisième.', 'Vérifie la règle des trois indices et propose où placer celui qui manque.', 'La machine tient la règle, le MJ garde le dernier mot.'),
    ('Les fiches', 'Relit le chef gobelin : stats en gemmes, actions en cartes.', 'A écrit les 14 fiches ; en régénère une sur demande.', 'Une fiche montre ce qui servira en jeu et ce qui reste caché.'),
    ('Cartes et médias', 'Lance les 4 illustrations utiles aux premières sessions.', 'Génère en arrière-plan, lieu par lieu, au prix annoncé.', 'Les médias coûtent cher : ils partent lot par lot, quand ils servent.'),
    ('Valider', 'Relit le bilan, copie le lien d’invitation, fixe la date.', 'Rien : la campagne devient jouable.', 'La préparation se ferme sur un rendez-vous avec les joueurs.'),
]

css = r'''
body{margin:0;background:#0A0A0A;overflow:hidden}
.pm{font-family:'Chakra Petch',system-ui,sans-serif;color:#F2F2F2;background:radial-gradient(120% 40% at 50% 0%,#1C1C1C 0,#0C0C0C 55%,#050505 100%);-webkit-font-smoothing:antialiased}
.ttl{font-family:'Cinzel',Georgia,serif;font-weight:800}
.lbl{font-size:11px;font-weight:600;letter-spacing:.2em;text-transform:uppercase;color:#8C8C8C}
.grid{display:grid;grid-template-columns:repeat(3,720px);gap:56px 48px}
.mo{display:flex;flex-direction:column;gap:14px}
.mh{display:flex;align-items:baseline;gap:10px}
.mh > b{font-family:'Cinzel',serif;font-size:13px;width:28px;height:28px;border-radius:7px;display:inline-grid;place-items:center;background:#EDEDED;color:#0A0A0A;box-shadow:0 2px 0 #6E6E6E;flex:none}
.mh > span{font-family:'Cinzel',serif;font-weight:800;font-size:20px}
.fr{width:720px;height:450px;overflow:hidden;border-radius:10px;border:1px solid #2C2C2C;box-shadow:0 20px 40px rgba(0,0,0,.6)}
.cap{display:grid;grid-template-columns:130px 1fr;gap:6px 10px;font-size:13px;line-height:1.45;color:#D4D4D4;margin:0}
.cap > dt{font-size:10px;font-weight:700;letter-spacing:.16em;text-transform:uppercase;color:#8C8C8C;padding-top:3px}
.cap > dd{margin:0}
.rule{display:flex;gap:12px;align-items:flex-start;font-size:13px;color:#D4D4D4;line-height:1.45}
.rule > b{flex:none;width:24px;height:24px;border-radius:6px;background:#EDEDED;color:#0A0A0A;display:grid;place-items:center;font-family:'Cinzel',serif;box-shadow:0 2px 0 #6E6E6E}
'''

W, H = 2384, 2760
frames = ''.join(
    f'<div class="mo"><div class="mh"><b>{i}</b><span>{t}</span></div>'
    f'<div class="fr"><div style="width: 1440px; height: 900px; transform: scale(.5); transform-origin: 0 0"><dc-import name="Preparer-MJ" moment="{i}" seul="true" hint-size="1440px,900px"></dc-import></div></div>'
    f'<dl class="cap"><dt>Le MJ</dt><dd>{does}</dd><dt>Le co-MJ</dt><dd>{secret}</dd><dt>Pourquoi</dt><dd>{why}</dd></dl></div>'
    for i, (t, does, secret, why) in enumerate(FRAMES, 1))

RULES = [
    ('Une étape à la fois.', 'Neuf étapes en haut de l’écran, un seul gros bouton pour passer à la suivante.'),
    ('Le prix avant l’appel.', 'Chaque génération affiche son coût avant de partir, et le budget de la campagne reste en vue.'),
    ('L’IA propose, le MJ valide.', 'Chaque changement du co-MJ est un diff que le MJ accepte, modifie ou rejette.'),
]
html = f'''<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8">
<title>Préparer côté MJ</title>
<script src="./support.js"></script>
</head>
<body>
<x-dc>
<helmet>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Cinzel:wght@600;700;800&amp;family=Chakra+Petch:wght@400;500;600;700&amp;family=Cormorant+Garamond:ital,wght@1,500;1,600&amp;display=swap">
<style>{css}</style>
</helmet>
<div class="pm" style="width: {W}px; height: {H}px; position: relative; overflow: hidden">
<div style="padding: 48px 64px 32px; display: flex; gap: 80px; align-items: flex-end">
<div style="display: flex; flex-direction: column; gap: 10px; max-width: 1000px">
<div class="lbl">MJ · ordinateur</div>
<h1 class="ttl" style="margin: 0; font-size: 50px">Préparer une campagne</h1>
<p style="margin: 0; font-size: 16px; color: #A3A3A3; line-height: 1.5">Romain prépare « Les Cendres de Valombre », la campagne que Marc jouera. Du choix de l’univers à la campagne jouable : le co-MJ écrit, Romain relit et tranche, et chaque appel payant affiche son prix avant de partir. La planche « Préparer » la rejoue en entier.</p>
</div>
<div style="display: flex; flex-direction: column; gap: 10px; max-width: 900px">
''' + ''.join(f'<div class="rule"><b>{n}</b><span><strong style="color: #F2F2F2">{a}</strong> {b}</span></div>' for n, (a, b) in enumerate(RULES, 1)) + f'''
</div>
</div>
<div class="grid" style="padding: 0 64px">{frames}</div>
</div>
</x-dc>
<script type="text/x-dc" data-dc-script data-props='{{"$preview":{{"width":{W},"height":{H}}}}}'>
class Component extends DCLogic {{
  renderVals() {{
    return {{}};
  }}
}}
</script>
</body>
</html>
'''
out = 'docs/design/canvas/Preparation-MJ.dc.html'
open(out, 'w').write(html)
d = json.load(open('docs/design/canvas/canvas.json'))
d['boards'].setdefault('Preparation-MJ.dc.html', {"x": 0, "y": 12000, "w": W, "h": H, "title": "MJ · préparer une campagne"})  # keep a layout set on the canvas
if 'Preparation-MJ.dc.html' not in d['order']: d['order'].append('Preparation-MJ.dc.html')
json.dump(d, open('docs/design/canvas/canvas.json', 'w'), ensure_ascii=False, indent=2)

sys.path.insert(0, 'docs/design')
from scope import scope, check
_out, _ren, _dyn = scope(open(out).read(), 'pmj-', keep=set())
open(out, 'w').write(_out)
print(len(_out), 'unsafe:', check(_out, 'pmj-', _dyn))
